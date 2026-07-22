use ids::{JobId, JobSliceId};
use sqlx::PgExecutor;

use super::JobSlice;

/// Insert `count` queued slices for a fanned-out job (indexes 0..count). Call ONLY when a
/// job actually fans out (n > 1) — single-call jobs have no slice rows.
pub async fn create_job_slices<'e, E: PgExecutor<'e>>(
  exec: E,
  job_id: &JobId,
  count: i32,
) -> sqlx::Result<Vec<JobSlice>> {
  let ids: Vec<String> = (0..count).map(|_| JobSliceId::generate().as_str().to_string()).collect();
  sqlx::query_as!(
    JobSlice,
    r#"INSERT INTO job_slices (id, job_id, slice_index, status)
       SELECT unnest($1::text[]), $2, generate_series(0, $3 - 1), 'queued'
       RETURNING id AS "id: ids::JobSliceId", job_id AS "job_id: ids::JobId", slice_index,
                 status AS "status: enums::JobSliceStatus", provider_job_id,
                 failure_code, failure_message, created_at, dispatched_at, completed_at"#,
    &ids,
    job_id.as_str(),
    count,
  )
  .fetch_all(exec)
  .await
}
