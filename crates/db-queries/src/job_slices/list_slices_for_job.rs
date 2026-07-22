use ids::JobId;
use sqlx::PgExecutor;

use super::JobSlice;

pub async fn list_slices_for_job<'e, E: PgExecutor<'e>>(exec: E, job_id: &JobId) -> sqlx::Result<Vec<JobSlice>> {
  sqlx::query_as!(
    JobSlice,
    r#"SELECT id AS "id: ids::JobSliceId", job_id AS "job_id: ids::JobId", slice_index,
                 status AS "status: enums::JobSliceStatus", provider_job_id,
                 failure_code, failure_message, created_at, dispatched_at, completed_at
       FROM job_slices WHERE job_id = $1 ORDER BY slice_index"#,
    job_id.as_str(),
  )
  .fetch_all(exec)
  .await
}
