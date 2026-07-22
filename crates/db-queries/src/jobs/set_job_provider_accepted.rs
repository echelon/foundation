use ids::JobId;
use sqlx::PgExecutor;

/// The provider accepted (upstream job created): record the handle, flip to
/// `running`/`at_provider`. For fanned-out jobs `provider_job_id` stays NULL here (the
/// slices carry the handles) — pass None.
pub async fn set_job_provider_accepted<'e, E: PgExecutor<'e>>(
  exec: E,
  id: &JobId,
  provider_job_id: Option<&str>,
) -> sqlx::Result<bool> {
  let result = sqlx::query!(
    r#"UPDATE jobs SET status = 'running', queue_stage = 'at_provider',
                       provider_job_id = $2, dispatched_at = now(), started_at = now()
       WHERE id = $1 AND status = 'queued'"#,
    id.as_str(),
    provider_job_id,
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected() > 0)
}
