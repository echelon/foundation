//! The embedded dispatcher: an in-process tokio loop that (a) fires queued jobs at the
//! provider and (b) polls running work back to completion, ingesting output bytes into R2.
//!
//! Deliberately in-process for this iteration — the claim query leases with
//! `FOR UPDATE SKIP LOCKED`, so extracting this into a standalone worker binary later is
//! a copy-paste, not a redesign. Fanned-out jobs (n>1) complete per SLICE: outputs appear
//! as each slice lands (partial completion), and the job goes terminal only when every
//! slice has — `succeeded` if at least one slice succeeded, `failed` only if all failed.

use std::time::Duration;

use sha2::{Digest, Sha256};

use bucket_client::UploadBytesArgs;
use bucket_paths::{mint_bucket_hash, object_key, BucketFolder, ObjectKeyParts};
use db_queries::{files, job_events, job_slices, jobs, outputs};
use enums::{JobEventType, JobSliceStatus, JobStatus};
use foundation_router::image::build::build_image_route;
use foundation_router::image::poll::{poll_openai_image, PollImageArgs, RoutedImage, RoutedSliceStatus};
use foundation_router::image::route::ImageRoute;
use serde_json::json;

use crate::generation::provider_log::{log_outbound_image_request, LogOutboundImageRequestArgs};
use crate::generation::stored_params::StoredImageParams;
use crate::state::AppState;

const TICK: Duration = Duration::from_secs(2);
/// Claim lease — a tick that dies mid-dispatch surrenders its jobs after this long.
const DISPATCH_LEASE_SECONDS: f64 = 120.0;
const DISPATCH_BATCH: i64 = 10;
const POLL_BATCH: i64 = 50;
/// Give up on dispatch after this many claim rounds (attempts count at claim time).
const MAX_DISPATCH_ATTEMPTS: i32 = 5;

pub fn spawn(state: AppState) {
  tokio::spawn(async move {
    let mut ticker = tokio::time::interval(TICK);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
      ticker.tick().await;
      if let Err(e) = dispatch_pass(&state).await {
        tracing::error!("dispatch pass errored: {e}");
      }
      if let Err(e) = poll_pass(&state).await {
        tracing::error!("poll pass errored: {e}");
      }
    }
  });
}

/// Backoff schedule after a failed dispatch attempt (attempt is 1-based).
fn backoff_seconds(attempt: i32) -> f64 {
  (2_f64).powi(attempt.min(6)) * 2.0 // 4s, 8s, 16s, 32s, 64s, capped
}

// ── Dispatch pass: queued → at the provider ──

async fn dispatch_pass(state: &AppState) -> sqlx::Result<()> {
  let claimed = jobs::claim_due_dispatch_jobs(&state.pool, DISPATCH_LEASE_SECONDS, DISPATCH_BATCH).await?;
  for job in claimed {
    dispatch_one(state, &job).await?;
  }
  Ok(())
}

async fn dispatch_one(state: &AppState, job: &jobs::Job) -> sqlx::Result<()> {
  job_events::create_job_event(
    &state.pool,
    &job.id,
    JobEventType::DispatchAttempted,
    Some(json!({ "attempt": job.dispatch_attempts })),
  )
  .await?;

  // Rebuild the route from the persisted (already-sanitized) params. A rebuild failure is
  // permanent — retrying identical inputs cannot help.
  let route = match rebuild_route(job) {
    Ok(route) => route,
    Err(reason) => return fail_dispatch_permanently(state, job, enums::FailureCode::Internal, &reason).await,
  };
  let Some(api_key) = state.openai.clone() else {
    return retry_or_fail(state, job, "provider not configured: no OpenAI credential").await;
  };

  if route.fan_out() == 1 {
    // Single-call job: the jobs row itself carries the provider handle. The provider_calls
    // row goes in BEFORE the send (its id is the request_id).
    let call = log_outbound_image_request(
      &state.pool,
      LogOutboundImageRequestArgs { job_id: &job.id, slice: None, route: &route, index: 0, is_dry_run: false },
    )
    .await?;
    let sent_at = std::time::Instant::now();
    match route.start_slice(&api_key, 0).await {
      Ok(handle) => {
        record_call_result(state, call.as_ref(), Ok(&handle.provider_job_id), sent_at).await?;
        jobs::set_job_provider_accepted(&state.pool, &job.id, Some(&handle.provider_job_id)).await?;
        job_events::create_job_event(
          &state.pool,
          &job.id,
          JobEventType::ProviderAccepted,
          Some(json!({ "provider_job_id": handle.provider_job_id })),
        )
        .await?;
      }
      Err(e) => {
        record_call_result(state, call.as_ref(), Err(&e.to_string()), sent_at).await?;
        return retry_or_fail(state, job, &e.to_string()).await;
      }
    }
    return Ok(());
  }

  // Fanned-out job: start every still-queued slice. Already-running slices (from a prior
  // partially-failed attempt) are skipped, so retries only touch what's missing.
  let slices = job_slices::list_slices_for_job(&state.pool, &job.id).await?;
  let mut start_errors: Vec<String> = Vec::new();
  for slice in &slices {
    if slice.status != JobSliceStatus::Queued || slice.provider_job_id.is_some() {
      continue;
    }
    let call = log_outbound_image_request(
      &state.pool,
      LogOutboundImageRequestArgs {
        job_id: &job.id,
        slice: Some(slice),
        route: &route,
        index: slice.slice_index as usize,
        is_dry_run: false,
      },
    )
    .await?;
    let sent_at = std::time::Instant::now();
    match route.start_slice(&api_key, slice.slice_index as usize).await {
      Ok(handle) => {
        record_call_result(state, call.as_ref(), Ok(&handle.provider_job_id), sent_at).await?;
        job_slices::set_slice_provider_accepted(&state.pool, &slice.id, &handle.provider_job_id).await?;
      }
      Err(e) => {
        record_call_result(state, call.as_ref(), Err(&e.to_string()), sent_at).await?;
        start_errors.push(format!("slice {}: {e}", slice.slice_index));
      }
    }
  }

  if start_errors.is_empty() {
    jobs::set_job_provider_accepted(&state.pool, &job.id, None).await?;
    job_events::create_job_event(
      &state.pool,
      &job.id,
      JobEventType::ProviderAccepted,
      Some(json!({ "slices": slices.len() })),
    )
    .await?;
    return Ok(());
  }

  if job.dispatch_attempts < MAX_DISPATCH_ATTEMPTS {
    // Some slices failed to start — leave the job queued; the next claim retries ONLY those.
    jobs::backoff_job_dispatch(&state.pool, &job.id, backoff_seconds(job.dispatch_attempts)).await?;
    return Ok(());
  }

  // Out of attempts: give up on the unstarted slices; keep the job alive if anything is
  // in flight (the poll pass finishes it), else fail it outright.
  let mut any_running = false;
  for slice in job_slices::list_slices_for_job(&state.pool, &job.id).await? {
    match slice.status {
      JobSliceStatus::Queued => {
        job_slices::complete_job_slice(
          &state.pool,
          job_slices::CompleteJobSlice {
            id: &slice.id,
            status: JobSliceStatus::Failed,
            failure_code: Some(enums::FailureCode::ProviderError),
            failure_message: Some("dispatch attempts exhausted"),
          },
        )
        .await?;
      }
      JobSliceStatus::Running => any_running = true,
      _ => {}
    }
  }
  if any_running {
    jobs::set_job_provider_accepted(&state.pool, &job.id, None).await?;
    Ok(())
  } else {
    fail_dispatch_permanently(state, job, enums::FailureCode::ProviderError, &start_errors.join("; ")).await
  }
}

/// Finish the pre-send provider_calls row with what came back (the provider handle) or
/// the send error. Logging must never fail the dispatch itself.
async fn record_call_result(
  state: &AppState,
  call: Option<&db_queries::ProviderCall>,
  outcome: Result<&str, &str>,
  sent_at: std::time::Instant,
) -> sqlx::Result<()> {
  let Some(call) = call else { return Ok(()) };
  let (provider_job_id, response_body) = match outcome {
    Ok(handle) => (Some(handle), json!({ "id": handle, "status": "queued" })),
    Err(e) => (None, json!({ "error": e })),
  };
  db_queries::provider_calls::set_provider_call_result(
    &state.pool,
    db_queries::provider_calls::ProviderCallResult {
      id: &call.id,
      provider_job_id,
      response_body: Some(&response_body),
      latency_ms: Some(sent_at.elapsed().as_millis().min(i32::MAX as u128) as i32),
    },
  )
  .await
}

fn rebuild_route(job: &jobs::Job) -> Result<ImageRoute, String> {
  let params: StoredImageParams =
    serde_json::from_value(job.request_params.clone()).map_err(|e| format!("stored params unreadable: {e}"))?;
  let plan = params.to_plan().ok_or_else(|| format!("model no longer routed: {}", params.model))?;
  let built = build_image_route(plan).map_err(|e| format!("route rebuild failed: {e}"))?;
  Ok(built.route)
}

async fn retry_or_fail(state: &AppState, job: &jobs::Job, error: &str) -> sqlx::Result<()> {
  if job.dispatch_attempts < MAX_DISPATCH_ATTEMPTS {
    tracing::warn!("dispatch of {} failed (attempt {}): {error}", job.id, job.dispatch_attempts);
    jobs::backoff_job_dispatch(&state.pool, &job.id, backoff_seconds(job.dispatch_attempts)).await
  } else {
    fail_dispatch_permanently(state, job, enums::FailureCode::ProviderError, error).await
  }
}

async fn fail_dispatch_permanently(
  state: &AppState,
  job: &jobs::Job,
  code: enums::FailureCode,
  message: &str,
) -> sqlx::Result<()> {
  tracing::error!("job {} failed permanently at dispatch: {message}", job.id);
  jobs::complete_job(
    &state.pool,
    jobs::CompleteJob {
      id: &job.id,
      status: JobStatus::Failed,
      failure_code: Some(code),
      failure_message: Some(message),
      canceled_by: None,
    },
  )
  .await?;
  job_events::create_job_event(&state.pool, &job.id, JobEventType::Failed, Some(json!({ "message": message })))
    .await
}

// ── Poll pass: at the provider → outputs ingested → terminal ──

async fn poll_pass(state: &AppState) -> sqlx::Result<()> {
  if state.openai.is_none() {
    return Ok(());
  }

  for job in jobs::list_pollable_unsliced_jobs(&state.pool, POLL_BATCH).await? {
    if let Err(e) = poll_and_finish_unsliced_job(state, &job).await {
      tracing::warn!("poll of {} failed (will retry): {e}", job.id);
    }
  }
  for slice in job_slices::list_pollable_slices(&state.pool, POLL_BATCH).await? {
    if let Err(e) = poll_and_finish_slice(state, &slice).await {
      tracing::warn!("poll of slice {} failed (will retry): {e}", slice.id);
    }
  }
  Ok(())
}

/// Fetch an UNSLICED job's provider state and advance it: ingest outputs on success, mark
/// failed on provider failure, no-op while pending. Called from the poll loop AND the
/// inbound-webhook handler (a webhook is just an early poll trigger — same code path).
/// Errors are retryable: the job stays `running` and the next tick tries again.
pub async fn poll_and_finish_unsliced_job(state: &AppState, job: &jobs::Job) -> Result<(), String> {
  let api_key = state.openai.clone().ok_or("no OpenAI credential configured")?;
  let handle = job.provider_job_id.clone().ok_or("job has no provider handle")?;
  let poll = poll_openai_image(PollImageArgs { api_key: &api_key, provider_job_id: &handle })
    .await
    .map_err(|e| e.to_string())?;
  match poll.status {
    RoutedSliceStatus::Pending => Ok(()),
    RoutedSliceStatus::Succeeded => ingest_and_complete_job(state, job, &poll.images).await,
    RoutedSliceStatus::Failed => {
      let message = poll.failure_message.as_deref().unwrap_or("provider reported failure");
      jobs::complete_job(
        &state.pool,
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
      job_events::create_job_event(&state.pool, &job.id, JobEventType::Failed, Some(json!({ "message": message })))
        .await
        .map_err(|e| e.to_string())
    }
  }
}

/// Slice twin of [`poll_and_finish_unsliced_job`] — advances one fan-out unit and, when it
/// was the last one out, lands the job (partial-completion rule lives in
/// [`finish_job_if_all_slices_terminal`]).
pub async fn poll_and_finish_slice(state: &AppState, slice: &db_queries::JobSlice) -> Result<(), String> {
  let api_key = state.openai.clone().ok_or("no OpenAI credential configured")?;
  let handle = slice.provider_job_id.clone().ok_or("slice has no provider handle")?;
  let poll = poll_openai_image(PollImageArgs { api_key: &api_key, provider_job_id: &handle })
    .await
    .map_err(|e| e.to_string())?;
  match poll.status {
    RoutedSliceStatus::Pending => Ok(()),
    RoutedSliceStatus::Succeeded => ingest_and_complete_slice(state, slice, &poll.images).await,
    RoutedSliceStatus::Failed => {
      let message = poll.failure_message.as_deref().unwrap_or("provider reported failure");
      job_slices::complete_job_slice(
        &state.pool,
        job_slices::CompleteJobSlice {
          id: &slice.id,
          status: JobSliceStatus::Failed,
          failure_code: Some(enums::FailureCode::ProviderError),
          failure_message: Some(message),
        },
      )
      .await
      .map_err(|e| e.to_string())?;
      finish_job_if_all_slices_terminal(state, &slice.job_id).await.map_err(|e| e.to_string())
    }
  }
}

/// Upload one routed image to R2 and record its `files` row. Upload happens OUTSIDE any tx
/// (an orphaned object behind an unguessable key is harmless; a dangling row is not).
async fn ingest_image(
  state: &AppState,
  account_id: &ids::AccountId,
  image: &RoutedImage,
) -> Result<db_queries::FileRow, String> {
  let bucket = state.bucket.as_ref().ok_or("output storage (R2) is not configured")?;
  let hash = mint_bucket_hash();
  let key = object_key(ObjectKeyParts {
    folder: BucketFolder::Generations,
    hash: &hash,
    prefix: None,
    suffix: None,
    version: None,
    extension: Some(image.extension),
  });
  bucket
    .upload_bytes(UploadBytesArgs { object_key: &key, bytes: &image.bytes, content_type: Some(image.mime) })
    .await
    .map_err(|e| format!("R2 upload failed: {e}"))?;
  files::create_file(
    &state.pool,
    files::NewFile {
      account_id,
      purpose: "output",
      origin: "generated",
      bucket_folder: BucketFolder::Generations.as_str(),
      bucket_hash: &hash,
      bucket_extension: Some(image.extension),
      mime: image.mime,
      bytes: image.bytes.len() as i64,
      checksum_sha256: &hex::encode(Sha256::digest(&image.bytes)),
    },
  )
  .await
  .map_err(|e| format!("files insert failed: {e}"))
}

async fn ingest_and_complete_job(state: &AppState, job: &jobs::Job, images: &[RoutedImage]) -> Result<(), String> {
  for (i, image) in images.iter().enumerate() {
    let file = ingest_image(state, &job.account_id, image).await?;
    outputs::create_output(
      &state.pool,
      outputs::NewOutput { job_id: &job.id, file_id: &file.id, output_index: i as i32, kind: "image", width: None, height: None },
    )
    .await
    .map_err(|e| format!("outputs insert failed: {e}"))?;
  }
  jobs::complete_job(
    &state.pool,
    jobs::CompleteJob { id: &job.id, status: JobStatus::Succeeded, failure_code: None, failure_message: None, canceled_by: None },
  )
  .await
  .map_err(|e| e.to_string())?;
  job_events::create_job_event(&state.pool, &job.id, JobEventType::OutputIngested, Some(json!({ "outputs": images.len() })))
    .await
    .map_err(|e| e.to_string())?;
  Ok(())
}

/// A fanned slice succeeded: ingest its (single) image at `output_index = slice_index`,
/// mark the slice, and finish the job if it was the last one out.
async fn ingest_and_complete_slice(
  state: &AppState,
  slice: &db_queries::JobSlice,
  images: &[RoutedImage],
) -> Result<(), String> {
  let job = jobs::find_job(&state.pool, &slice.job_id)
    .await
    .map_err(|e| e.to_string())?
    .ok_or("job row vanished")?;
  if let Some(image) = images.first() {
    let file = ingest_image(state, &job.account_id, image).await?;
    outputs::create_output(
      &state.pool,
      outputs::NewOutput {
        job_id: &job.id,
        file_id: &file.id,
        output_index: slice.slice_index,
        kind: "image",
        width: None,
        height: None,
      },
    )
    .await
    .map_err(|e| format!("outputs insert failed: {e}"))?;
  }
  job_slices::complete_job_slice(
    &state.pool,
    job_slices::CompleteJobSlice { id: &slice.id, status: JobSliceStatus::Succeeded, failure_code: None, failure_message: None },
  )
  .await
  .map_err(|e| e.to_string())?;
  finish_job_if_all_slices_terminal(state, &slice.job_id).await.map_err(|e| e.to_string())?;
  Ok(())
}

/// Fan-out completion rule: terminal when EVERY slice is; `succeeded` if any slice
/// succeeded (partial results are results), `failed` only when all failed.
async fn finish_job_if_all_slices_terminal(state: &AppState, job_id: &ids::JobId) -> sqlx::Result<()> {
  let slices = job_slices::list_slices_for_job(&state.pool, job_id).await?;
  if slices.is_empty() || !slices.iter().all(|s| s.status.is_terminal()) {
    return Ok(());
  }
  let succeeded = slices.iter().filter(|s| s.status == JobSliceStatus::Succeeded).count();
  let (status, code, message) = if succeeded > 0 {
    (JobStatus::Succeeded, None, None)
  } else {
    (JobStatus::Failed, Some(enums::FailureCode::ProviderError), Some("all slices failed"))
  };
  let flipped = jobs::complete_job(
    &state.pool,
    jobs::CompleteJob { id: job_id, status, failure_code: code, failure_message: message, canceled_by: None },
  )
  .await?;
  if flipped {
    job_events::create_job_event(
      &state.pool,
      job_id,
      if succeeded > 0 { JobEventType::OutputIngested } else { JobEventType::Failed },
      Some(json!({ "slices_succeeded": succeeded, "slices_total": slices.len() })),
    )
    .await?;
  }
  Ok(())
}
