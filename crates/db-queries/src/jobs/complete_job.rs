use enums::{CanceledBy, FailureCode, JobStatus};
use ids::JobId;
use sqlx::PgExecutor;

#[derive(Debug)]
pub struct CompleteJob<'a> {
  pub id: &'a JobId,
  /// A terminal status (succeeded / failed / canceled / expired).
  pub status: JobStatus,
  pub failure_code: Option<FailureCode>,
  pub failure_message: Option<&'a str>,
  /// Who canceled, when `status` is `canceled` (jobs.md: a field, not extra statuses).
  pub canceled_by: Option<CanceledBy>,
}

/// Terminal transition. Only fires from a non-terminal state (idempotent under races).
pub async fn complete_job<'e, E: PgExecutor<'e>>(exec: E, args: CompleteJob<'_>) -> sqlx::Result<bool> {
  let result = sqlx::query!(
    r#"UPDATE jobs SET status = $2, failure_code = $3, failure_message = $4, canceled_by = $5,
                       completed_at = now()
       WHERE id = $1 AND status IN ('queued', 'running')"#,
    args.id.as_str(),
    args.status.as_str(),
    args.failure_code.map(|c| c.as_str()),
    args.failure_message,
    args.canceled_by.map(|c| c.as_str()),
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected() > 0)
}
