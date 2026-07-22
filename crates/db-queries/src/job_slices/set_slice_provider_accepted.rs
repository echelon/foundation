use ids::JobSliceId;
use sqlx::PgExecutor;

pub async fn set_slice_provider_accepted<'e, E: PgExecutor<'e>>(
  exec: E,
  id: &JobSliceId,
  provider_job_id: &str,
) -> sqlx::Result<bool> {
  let result = sqlx::query!(
    r#"UPDATE job_slices SET status = 'running', provider_job_id = $2, dispatched_at = now()
       WHERE id = $1 AND status = 'queued'"#,
    id.as_str(),
    provider_job_id,
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected() > 0)
}
