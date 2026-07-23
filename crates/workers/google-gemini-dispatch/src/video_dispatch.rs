//! Dispatch ONE claimed Google VIDEO job. Veo is ASYNC at the provider: this
//! flow STARTS one operation per slice (uploading the i2v frame first, when
//! present) and records the handles; the polling job lands the results.

use db_queries::{job_events, job_slices, jobs};
use enums::{JobEventType, JobSliceStatus};
use serde_json::json;

use crate::complete::{
  backoff_seconds, fail_job_permanently, fail_slice, retry_or_fail, MAX_DISPATCH_ATTEMPTS,
};
use crate::context::DispatchContext;
use crate::inputs::prepare_video_input;
use crate::provider_log::{log_outbound_request, record_call_result, LogOutboundRequestArgs, RouteRef};
use crate::rebuild::rebuild_video_route;

pub async fn dispatch_video_job(ctx: &DispatchContext, job: &jobs::Job) -> sqlx::Result<()> {
  job_events::create_job_event(
    &ctx.pool,
    &job.id,
    JobEventType::DispatchAttempted,
    Some(json!({ "attempt": job.dispatch_attempts })),
  )
  .await?;

  let route = match rebuild_video_route(job) {
    Ok(route) => route,
    Err(reason) => return fail_job_permanently(&ctx.pool, &job.id, enums::FailureCode::Internal, &reason).await,
  };

  // The i2v conditioning frame (if any) prepares once, shared by every slice —
  // inline bytes for Veo, a Files-API ref for Omni.
  let image = match prepare_video_input(ctx, route.input_url(), route.prefers_files_api_input()).await {
    Ok(image) => image,
    Err(e) => return retry_or_fail(&ctx.pool, job, &e).await,
  };

  if route.fan_out() == 1 {
    let call = log_outbound_request(
      &ctx.pool,
      LogOutboundRequestArgs { job_id: &job.id, slice: None, route: RouteRef::Video(&route), index: 0 },
    )
    .await?;
    let sent_at = std::time::Instant::now();
    match route.start_google_slice(&ctx.api_key, 0, image).await {
      Ok(handle) => {
        let body = json!({ "id": handle.provider_job_id, "status": "queued" });
        record_call_result(&ctx.pool, call.as_ref(), Ok(&body), sent_at).await?;
        jobs::set_job_provider_accepted(&ctx.pool, &job.id, Some(&handle.provider_job_id)).await?;
        job_events::create_job_event(
          &ctx.pool,
          &job.id,
          JobEventType::ProviderAccepted,
          Some(json!({ "provider_job_id": handle.provider_job_id })),
        )
        .await?;
      }
      Err(e) => {
        record_call_result(&ctx.pool, call.as_ref(), Err(&e.to_string()), sent_at).await?;
        return retry_or_fail(&ctx.pool, job, &e.to_string()).await;
      }
    }
    return Ok(());
  }

  // Fanned-out: start every still-queued slice; retries only touch what's missing.
  let slices = job_slices::list_slices_for_job(&ctx.pool, &job.id).await?;
  let mut start_errors: Vec<String> = Vec::new();
  for slice in &slices {
    if slice.status != JobSliceStatus::Queued || slice.provider_job_id.is_some() {
      continue;
    }
    let index = slice.slice_index as usize;
    let call = log_outbound_request(
      &ctx.pool,
      LogOutboundRequestArgs { job_id: &job.id, slice: Some(slice), route: RouteRef::Video(&route), index },
    )
    .await?;
    let sent_at = std::time::Instant::now();
    match route.start_google_slice(&ctx.api_key, index, image.clone()).await {
      Ok(handle) => {
        let body = json!({ "id": handle.provider_job_id, "status": "queued" });
        record_call_result(&ctx.pool, call.as_ref(), Ok(&body), sent_at).await?;
        job_slices::set_slice_provider_accepted(&ctx.pool, &slice.id, &handle.provider_job_id).await?;
      }
      Err(e) => {
        record_call_result(&ctx.pool, call.as_ref(), Err(&e.to_string()), sent_at).await?;
        start_errors.push(format!("slice {}: {e}", slice.slice_index));
      }
    }
  }

  if start_errors.is_empty() {
    jobs::set_job_provider_accepted(&ctx.pool, &job.id, None).await?;
    job_events::create_job_event(
      &ctx.pool,
      &job.id,
      JobEventType::ProviderAccepted,
      Some(json!({ "slices": slices.len() })),
    )
    .await?;
    return Ok(());
  }

  if job.dispatch_attempts < MAX_DISPATCH_ATTEMPTS {
    jobs::backoff_job_dispatch(&ctx.pool, &job.id, backoff_seconds(job.dispatch_attempts)).await?;
    return Ok(());
  }

  // Out of attempts: give up on unstarted slices; keep the job alive if anything
  // is in flight (the polling job finishes it), else fail it outright.
  let mut any_running = false;
  for slice in job_slices::list_slices_for_job(&ctx.pool, &job.id).await? {
    match slice.status {
      JobSliceStatus::Queued => fail_slice(&ctx.pool, &slice.id, "dispatch attempts exhausted").await?,
      JobSliceStatus::Running => any_running = true,
      _ => {}
    }
  }
  if any_running {
    jobs::set_job_provider_accepted(&ctx.pool, &job.id, None).await?;
    Ok(())
  } else {
    fail_job_permanently(&ctx.pool, &job.id, enums::FailureCode::ProviderError, &start_errors.join("; ")).await
  }
}
