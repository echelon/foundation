use ids::JobId;
use sqlx::PgExecutor;

/// Dispatch failed transiently — retry after `delay_seconds` (attempts already counted at
/// claim time; jobs.md: invisible retries under stable `queued`).
pub async fn backoff_job_dispatch<'e, E: PgExecutor<'e>>(
  exec: E,
  id: &JobId,
  delay_seconds: f64,
) -> sqlx::Result<()> {
  sqlx::query!(
    r#"UPDATE jobs SET next_dispatch_at = now() + make_interval(secs => $2)
       WHERE id = $1 AND status = 'queued'"#,
    id.as_str(),
    delay_seconds,
  )
  .execute(exec)
  .await?;
  Ok(())
}
