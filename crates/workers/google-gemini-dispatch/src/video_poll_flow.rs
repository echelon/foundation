//! Poll started Google video work to completion (Veo operations AND Omni
//! interactions — the router's `poll_google_video` hides which surface):
//! fetch state, download finished videos, ingest to R2, land the job/slice
//! (partial-completion rule applies).

use db_queries::{job_events, jobs};
use enums::{JobEventType, JobStatus};
use foundation_router::image::poll::RoutedSliceStatus;
use foundation_router::video::plan::RouterVideoModel;
use foundation_router::video::poll::{fetch_video_payload, poll_google_video, PollVideoArgs, RoutedVideoPoll};
use serde_json::json;

use crate::complete::{fail_slice, finish_job_if_all_slices_terminal, succeed_slice};
use crate::context::DispatchContext;
use crate::ingest::{ingest_output, IngestOutputArgs};

/// Give up on a provider operation that never lands. Needed in practice: Omni
/// background interactions occasionally come back born `queued` and their GET
/// then returns 400 "invalid argument" FOREVER (live-observed provider bug) —
/// without a deadline such jobs would hang until the 24 h expiry.
const VIDEO_POLL_DEADLINE_MINUTES: i64 = 30;

fn past_deadline(since: chrono::DateTime<chrono::Utc>) -> bool {
  chrono::Utc::now() - since > chrono::Duration::minutes(VIDEO_POLL_DEADLINE_MINUTES)
}

/// Which router model a job's handle belongs to (picks the poll surface).
fn model_for(job: &jobs::Job) -> Result<RouterVideoModel, String> {
  RouterVideoModel::from_slug(&job.model).ok_or_else(|| format!("model no longer routed: {}", job.model))
}

/// Bucket-path extension for a downloaded video mime (Gemini emits mp4/webm).
fn video_extension(mime: &str) -> &'static str {
  if mime == "video/webm" {
    ".webm"
  } else {
    ".mp4"
  }
}

async fn ingest_videos(
  ctx: &DispatchContext,
  job: &jobs::Job,
  poll: RoutedVideoPoll,
  base_index: i32,
) -> Result<usize, String> {
  let count = poll.videos.len();
  for (i, payload) in poll.videos.into_iter().enumerate() {
    let (mime, bytes) = fetch_video_payload(&ctx.api_key, payload).await.map_err(|e| e.to_string())?;
    ingest_output(
      ctx,
      IngestOutputArgs {
        account_id: &job.account_id,
        job_id: &job.id,
        output_index: base_index + i as i32,
        kind: "video",
        bytes: &bytes,
        mime: &mime,
        extension: video_extension(&mime),
      },
    )
    .await?;
  }
  Ok(count)
}

/// Advance one UNSLICED (n = 1) running video job. Errors are retryable: the
/// job stays `running` and the next tick tries again.
pub async fn poll_and_finish_unsliced_video_job(ctx: &DispatchContext, job: &jobs::Job) -> Result<(), String> {
  if past_deadline(job.created_at) {
    return crate::complete::fail_job_permanently(
      &ctx.pool,
      &job.id,
      enums::FailureCode::ProviderError,
      "provider did not complete the video within the deadline",
    )
    .await
    .map_err(|e| e.to_string());
  }
  let handle = job.provider_job_id.clone().ok_or("job has no provider handle")?;
  let poll =
    poll_google_video(PollVideoArgs { api_key: &ctx.api_key, model: model_for(job)?, provider_job_id: &handle })
      .await
      .map_err(|e| e.to_string())?;
  match poll.status {
    RoutedSliceStatus::Pending => Ok(()),
    RoutedSliceStatus::Succeeded => {
      let count = ingest_videos(ctx, job, poll, 0).await?;
      crate::complete::succeed_job(&ctx.pool, &job.id, count).await.map_err(|e| e.to_string())
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
  if past_deadline(slice.dispatched_at.unwrap_or(slice.created_at)) {
    fail_slice(&ctx.pool, &slice.id, "provider did not complete the video within the deadline")
      .await
      .map_err(|e| e.to_string())?;
    return finish_job_if_all_slices_terminal(&ctx.pool, &slice.job_id).await.map_err(|e| e.to_string());
  }
  let handle = slice.provider_job_id.clone().ok_or("slice has no provider handle")?;
  let job = jobs::find_job(&ctx.pool, &slice.job_id)
    .await
    .map_err(|e| e.to_string())?
    .ok_or("job row vanished")?;
  let poll =
    poll_google_video(PollVideoArgs { api_key: &ctx.api_key, model: model_for(&job)?, provider_job_id: &handle })
      .await
      .map_err(|e| e.to_string())?;
  match poll.status {
    RoutedSliceStatus::Pending => Ok(()),
    RoutedSliceStatus::Succeeded => {
      ingest_videos(ctx, &job, poll, slice.slice_index).await?;
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
