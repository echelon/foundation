//! Terminal-state bookkeeping shared by both worker binaries: retry backoff,
//! permanent failure, and the partial-completion rule (jobs.md: a fanned job
//! `succeeded` if ANY slice succeeded; `failed` only when all failed).

use db_queries::{job_events, job_slices, jobs};
use enums::{JobEventType, JobSliceStatus, JobStatus};
use serde_json::json;
use sqlx::PgPool;

/// Give up on dispatch after this many claim rounds (attempts count at claim time).
pub const MAX_DISPATCH_ATTEMPTS: i32 = 5;

/// Backoff schedule after a failed dispatch attempt (attempt is 1-based).
pub fn backoff_seconds(attempt: i32) -> f64 {
  (2_f64).powi(attempt.min(6)) * 2.0 // 4s, 8s, 16s, 32s, 64s, capped
}

pub async fn retry_or_fail(pool: &PgPool, job: &jobs::Job, error: &str) -> sqlx::Result<()> {
  if job.dispatch_attempts < MAX_DISPATCH_ATTEMPTS {
    tracing::warn!("dispatch of {} failed (attempt {}): {error}", job.id, job.dispatch_attempts);
    jobs::backoff_job_dispatch(pool, &job.id, backoff_seconds(job.dispatch_attempts)).await
  } else {
    fail_job_permanently(pool, &job.id, enums::FailureCode::ProviderError, error).await
  }
}

pub async fn fail_job_permanently(
  pool: &PgPool,
  job_id: &ids::JobId,
  code: enums::FailureCode,
  message: &str,
) -> sqlx::Result<()> {
  tracing::error!("job {job_id} failed permanently: {message}");
  jobs::complete_job(
    pool,
    jobs::CompleteJob {
      id: job_id,
      status: JobStatus::Failed,
      failure_code: Some(code),
      failure_message: Some(message),
      canceled_by: None,
    },
  )
  .await?;
  job_events::create_job_event(pool, job_id, JobEventType::Failed, Some(json!({ "message": message }))).await
}

pub async fn succeed_job(pool: &PgPool, job_id: &ids::JobId, outputs: usize) -> sqlx::Result<()> {
  jobs::complete_job(
    pool,
    jobs::CompleteJob { id: job_id, status: JobStatus::Succeeded, failure_code: None, failure_message: None, canceled_by: None },
  )
  .await?;
  job_events::create_job_event(pool, job_id, JobEventType::OutputIngested, Some(json!({ "outputs": outputs }))).await
}

pub async fn succeed_slice(pool: &PgPool, slice_id: &ids::JobSliceId) -> sqlx::Result<()> {
  job_slices::complete_job_slice(
    pool,
    job_slices::CompleteJobSlice { id: slice_id, status: JobSliceStatus::Succeeded, failure_code: None, failure_message: None },
  )
  .await?;
  Ok(())
}

pub async fn fail_slice(pool: &PgPool, slice_id: &ids::JobSliceId, message: &str) -> sqlx::Result<()> {
  job_slices::complete_job_slice(
    pool,
    job_slices::CompleteJobSlice {
      id: slice_id,
      status: JobSliceStatus::Failed,
      failure_code: Some(enums::FailureCode::ProviderError),
      failure_message: Some(message),
    },
  )
  .await?;
  Ok(())
}

/// Fan-out completion rule: terminal when EVERY slice is; `succeeded` if any
/// slice succeeded (partial results are results), `failed` only when all failed.
pub async fn finish_job_if_all_slices_terminal(pool: &PgPool, job_id: &ids::JobId) -> sqlx::Result<()> {
  let slices = job_slices::list_slices_for_job(pool, job_id).await?;
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
    pool,
    jobs::CompleteJob { id: job_id, status, failure_code: code, failure_message: message, canceled_by: None },
  )
  .await?;
  if flipped {
    job_events::create_job_event(
      pool,
      job_id,
      if succeeded > 0 { JobEventType::OutputIngested } else { JobEventType::Failed },
      Some(json!({ "slices_succeeded": succeeded, "slices_total": slices.len() })),
    )
    .await?;
  }
  Ok(())
}
