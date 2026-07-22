use enums::{FailureCode, JobSliceStatus};
use ids::JobSliceId;
use sqlx::PgExecutor;

#[derive(Debug)]
pub struct CompleteJobSlice<'a> {
  pub id: &'a JobSliceId,
  pub status: JobSliceStatus,
  pub failure_code: Option<FailureCode>,
  pub failure_message: Option<&'a str>,
}

pub async fn complete_job_slice<'e, E: PgExecutor<'e>>(exec: E, args: CompleteJobSlice<'_>) -> sqlx::Result<bool> {
  let result = sqlx::query!(
    r#"UPDATE job_slices SET status = $2, failure_code = $3, failure_message = $4, completed_at = now()
       WHERE id = $1 AND status IN ('queued', 'running')"#,
    args.id.as_str(),
    args.status.as_str(),
    args.failure_code.map(|c| c.as_str()),
    args.failure_message,
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected() > 0)
}
