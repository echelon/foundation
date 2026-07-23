//! Dispatch ONE claimed Google IMAGE job. Nano Banana is synchronous at the
//! provider: running a slice completes it — inputs are uploaded first (Files
//! API), then each slice call returns bytes we ingest immediately.

use db_queries::{job_events, job_slices, jobs};
use enums::{JobEventType, JobSliceStatus};
use serde_json::json;

use crate::complete::{
  fail_job_permanently, finish_job_if_all_slices_terminal, retry_or_fail, succeed_job, succeed_slice,
  MAX_DISPATCH_ATTEMPTS,
};
use crate::context::DispatchContext;
use crate::ingest::{ingest_output, IngestOutputArgs};
use crate::inputs::upload_image_inputs;
use crate::provider_log::{log_outbound_request, record_call_result, LogOutboundRequestArgs, RouteRef};
use crate::rebuild::rebuild_image_route;

pub async fn dispatch_image_job(ctx: &DispatchContext, job: &jobs::Job) -> sqlx::Result<()> {
  job_events::create_job_event(
    &ctx.pool,
    &job.id,
    JobEventType::DispatchAttempted,
    Some(json!({ "attempt": job.dispatch_attempts })),
  )
  .await?;

  let route = match rebuild_image_route(job) {
    Ok(route) => route,
    Err(reason) => return fail_job_permanently(&ctx.pool, &job.id, enums::FailureCode::Internal, &reason).await,
  };

  // Upload-first inputs: one Files-API upload per input url, shared by every slice.
  let inputs = match upload_image_inputs(ctx, route.input_urls()).await {
    Ok(inputs) => inputs,
    Err(e) => return retry_or_fail(&ctx.pool, job, &e).await,
  };

  // NOTE: the job stays `queued` while the synchronous calls grind — flipping to
  // `running` would break re-claim (claim + backoff both require `queued`). The
  // claim lease keeps other workers off it; outputs/slices still land as we go.

  if route.fan_out() == 1 {
    let call = log_outbound_request(
      &ctx.pool,
      LogOutboundRequestArgs { job_id: &job.id, slice: None, route: RouteRef::Image(&route), index: 0 },
    )
    .await?;
    let sent_at = std::time::Instant::now();
    match route.run_google_slice(&ctx.api_key, 0, &inputs).await {
      Ok(images) => {
        let body = json!({ "images": images.len(), "status": "succeeded" });
        record_call_result(&ctx.pool, call.as_ref(), Ok(&body), sent_at).await?;
        for (i, image) in images.iter().enumerate() {
          if let Err(e) = ingest_output(
            ctx,
            IngestOutputArgs {
              account_id: &job.account_id,
              job_id: &job.id,
              output_index: i as i32,
              kind: "image",
              bytes: &image.bytes,
              mime: image.mime,
              extension: image.extension,
            },
          )
          .await
          {
            return retry_or_fail(&ctx.pool, job, &e).await;
          }
        }
        succeed_job(&ctx.pool, &job.id, images.len()).await?;
      }
      Err(e) => {
        record_call_result(&ctx.pool, call.as_ref(), Err(&e.to_string()), sent_at).await?;
        return retry_or_fail(&ctx.pool, job, &e.to_string()).await;
      }
    }
    return Ok(());
  }

  // Fanned-out: run every still-queued slice to completion; a retry round only
  // touches what's missing.
  let slices = job_slices::list_slices_for_job(&ctx.pool, &job.id).await?;
  let mut run_errors: Vec<String> = Vec::new();
  for slice in &slices {
    if slice.status != JobSliceStatus::Queued {
      continue;
    }
    let index = slice.slice_index as usize;
    let call = log_outbound_request(
      &ctx.pool,
      LogOutboundRequestArgs { job_id: &job.id, slice: Some(slice), route: RouteRef::Image(&route), index },
    )
    .await?;
    let sent_at = std::time::Instant::now();
    match route.run_google_slice(&ctx.api_key, index, &inputs).await {
      Ok(images) => {
        let body = json!({ "images": images.len(), "status": "succeeded" });
        record_call_result(&ctx.pool, call.as_ref(), Ok(&body), sent_at).await?;
        let mut ingest_error = None;
        if let Some(image) = images.first() {
          if let Err(e) = ingest_output(
            ctx,
            IngestOutputArgs {
              account_id: &job.account_id,
              job_id: &job.id,
              output_index: slice.slice_index,
              kind: "image",
              bytes: &image.bytes,
              mime: image.mime,
              extension: image.extension,
            },
          )
          .await
          {
            ingest_error = Some(e);
          }
        }
        match ingest_error {
          None => succeed_slice(&ctx.pool, &slice.id).await?,
          // Ingest failed — leave the slice queued so a later round retries it.
          Some(e) => run_errors.push(format!("slice {}: {e}", slice.slice_index)),
        }
      }
      Err(e) => {
        record_call_result(&ctx.pool, call.as_ref(), Err(&e.to_string()), sent_at).await?;
        run_errors.push(format!("slice {}: {e}", slice.slice_index));
      }
    }
  }

  if run_errors.is_empty() {
    return finish_job_if_all_slices_terminal(&ctx.pool, &job.id).await;
  }

  if job.dispatch_attempts < MAX_DISPATCH_ATTEMPTS {
    // Some slices failed this round — requeue; the next claim retries ONLY those.
    tracing::warn!("job {} slice failures (attempt {}): {}", job.id, job.dispatch_attempts, run_errors.join("; "));
    return jobs::backoff_job_dispatch(&ctx.pool, &job.id, crate::complete::backoff_seconds(job.dispatch_attempts))
      .await;
  }

  // Out of attempts: fail whatever is still queued, then apply the partial rule.
  for slice in job_slices::list_slices_for_job(&ctx.pool, &job.id).await? {
    if slice.status == JobSliceStatus::Queued {
      crate::complete::fail_slice(&ctx.pool, &slice.id, "dispatch attempts exhausted").await?;
    }
  }
  finish_job_if_all_slices_terminal(&ctx.pool, &job.id).await
}
