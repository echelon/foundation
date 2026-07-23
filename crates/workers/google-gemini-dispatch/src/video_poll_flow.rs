//! Poll started Veo operations to completion: fetch state, download finished
//! videos, ingest to R2, land the job/slice (partial-completion rule applies).

use db_queries::{job_events, jobs};
use enums::{JobEventType, JobStatus};
use foundation_router::image::poll::RoutedSliceStatus;
use foundation_router::video::poll::{download_routed_video, poll_google_video, PollVideoArgs};
use serde_json::json;

use crate::complete::{fail_slice, finish_job_if_all_slices_terminal, succeed_slice};
use crate::context::DispatchContext;
use crate::ingest::{ingest_output, IngestOutputArgs};

/// Advance one UNSLICED (n = 1) running video job. Errors are retryable: the
/// job stays `running` and the next tick tries again.
pub async fn poll_and_finish_unsliced_video_job(ctx: &DispatchContext, job: &jobs::Job) -> Result<(), String> {
  let handle = job.provider_job_id.clone().ok_or("job has no provider handle")?;
  let poll = poll_google_video(PollVideoArgs { api_key: &ctx.api_key, provider_job_id: &handle })
    .await
    .map_err(|e| e.to_string())?;
  match poll.status {
    RoutedSliceStatus::Pending => Ok(()),
    RoutedSliceStatus::Succeeded => {
      for (i, uri) in poll.video_uris.iter().enumerate() {
        let bytes = download_routed_video(&ctx.api_key, uri).await.map_err(|e| e.to_string())?;
        ingest_output(
          ctx,
          IngestOutputArgs {
            account_id: &job.account_id,
            job_id: &job.id,
            output_index: i as i32,
            kind: "video",
            bytes: &bytes,
            mime: "video/mp4",
            extension: ".mp4",
          },
        )
        .await?;
      }
      crate::complete::succeed_job(&ctx.pool, &job.id, poll.video_uris.len()).await.map_err(|e| e.to_string())
    }
    RoutedSliceStatus::Failed => {
      let message = poll.failure_message.as_deref().unwrap_or("provider reported failure");
      jobs::complete_job(
        &ctx.pool,
        jobs::CompleteJob {
          id: &job.id,
          status: JobStatus::Failed,
          failure_code: Some(enums::FailureCode::ProviderError),
          failure_message: Some(message),
          canceled_by: None,
        },
      )
      .await
      .map_err(|e| e.to_string())?;
      job_events::create_job_event(&ctx.pool, &job.id, JobEventType::Failed, Some(json!({ "message": message })))
        .await
        .map_err(|e| e.to_string())
    }
  }
}

/// Slice twin — advances one fan-out unit and, when it was the last one out,
/// lands the job (`succeeded` if ANY slice succeeded).
pub async fn poll_and_finish_video_slice(ctx: &DispatchContext, slice: &db_queries::JobSlice) -> Result<(), String> {
  let handle = slice.provider_job_id.clone().ok_or("slice has no provider handle")?;
  let poll = poll_google_video(PollVideoArgs { api_key: &ctx.api_key, provider_job_id: &handle })
    .await
    .map_err(|e| e.to_string())?;
  match poll.status {
    RoutedSliceStatus::Pending => Ok(()),
    RoutedSliceStatus::Succeeded => {
      let job = jobs::find_job(&ctx.pool, &slice.job_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("job row vanished")?;
      if let Some(uri) = poll.video_uris.first() {
        let bytes = download_routed_video(&ctx.api_key, uri).await.map_err(|e| e.to_string())?;
        ingest_output(
          ctx,
          IngestOutputArgs {
            account_id: &job.account_id,
            job_id: &job.id,
            output_index: slice.slice_index,
            kind: "video",
            bytes: &bytes,
            mime: "video/mp4",
            extension: ".mp4",
          },
        )
        .await?;
      }
      succeed_slice(&ctx.pool, &slice.id).await.map_err(|e| e.to_string())?;
      finish_job_if_all_slices_terminal(&ctx.pool, &slice.job_id).await.map_err(|e| e.to_string())
    }
    RoutedSliceStatus::Failed => {
      let message = poll.failure_message.as_deref().unwrap_or("provider reported failure");
      fail_slice(&ctx.pool, &slice.id, message).await.map_err(|e| e.to_string())?;
      finish_job_if_all_slices_terminal(&ctx.pool, &slice.job_id).await.map_err(|e| e.to_string())
    }
  }
}
