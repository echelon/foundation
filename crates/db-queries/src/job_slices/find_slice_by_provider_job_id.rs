use sqlx::PgExecutor;

use super::JobSlice;

/// Slice by its provider handle — how a webhook finds the fan-out unit it's about.
pub async fn find_slice_by_provider_job_id<'e, E: PgExecutor<'e>>(
  exec: E,
  provider_job_id: &str,
) -> sqlx::Result<Option<JobSlice>> {
  sqlx::query_as!(
    JobSlice,
    r#"SELECT id AS "id: ids::JobSliceId", job_id AS "job_id: ids::JobId", slice_index,
                 status AS "status: enums::JobSliceStatus", provider_job_id,
                 failure_code, failure_message, created_at, dispatched_at, completed_at
       FROM job_slices WHERE provider_job_id = $1
       ORDER BY sid DESC LIMIT 1"#,
    provider_job_id,
  )
  .fetch_optional(exec)
  .await
}
