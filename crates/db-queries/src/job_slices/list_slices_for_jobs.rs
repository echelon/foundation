use ids::JobId;
use sqlx::PgExecutor;

use super::JobSlice;

/// Batch twin of [`list_slices_for_job`] — one round-trip for a whole listing page
/// (callers group by `job_id` in memory).
pub async fn list_slices_for_jobs<'e, E: PgExecutor<'e>>(exec: E, job_ids: &[JobId]) -> sqlx::Result<Vec<JobSlice>> {
  let ids: Vec<String> = job_ids.iter().map(|j| j.as_str().to_string()).collect();
  sqlx::query_as!(
    JobSlice,
    r#"SELECT id AS "id: ids::JobSliceId", job_id AS "job_id: ids::JobId", slice_index,
                 status AS "status: enums::JobSliceStatus", provider_job_id,
                 failure_code, failure_message, created_at, dispatched_at, completed_at
       FROM job_slices WHERE job_id = ANY($1) ORDER BY job_id, slice_index"#,
    &ids,
  )
  .fetch_all(exec)
  .await
}
