use sqlx::PgExecutor;

use super::JobSlice;

/// Running slices with provider handles, poll-due across all jobs.
pub async fn list_pollable_slices<'e, E: PgExecutor<'e>>(exec: E, limit: i64) -> sqlx::Result<Vec<JobSlice>> {
  sqlx::query_as!(
    JobSlice,
    r#"SELECT id AS "id: ids::JobSliceId", job_id AS "job_id: ids::JobId", slice_index,
                 status AS "status: enums::JobSliceStatus", provider_job_id,
                 failure_code, failure_message, created_at, dispatched_at, completed_at
       FROM job_slices
       WHERE status = 'running' AND provider_job_id IS NOT NULL
       ORDER BY sid
       LIMIT $1"#,
    limit,
  )
  .fetch_all(exec)
  .await
}
