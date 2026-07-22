use ids::JobId;
use sqlx::PgExecutor;

use super::ProviderCall;

/// Every outbound call a job produced (fan-out jobs have one per slice) — the debug view.
pub async fn list_provider_calls_for_job<'e, E: PgExecutor<'e>>(
  exec: E,
  job_id: &JobId,
) -> sqlx::Result<Vec<ProviderCall>> {
  sqlx::query_as!(
    ProviderCall,
    r#"SELECT id AS "id: ids::ProviderCallId", job_id AS "job_id: ids::JobId",
                 job_slice_id AS "job_slice_id?: ids::JobSliceId", provider, direction, is_dry_run,
                 provider_job_id, url, request_body, response_status, response_body, latency_ms,
                 created_at
       FROM provider_calls WHERE job_id = $1 ORDER BY sid"#,
    job_id.as_str(),
  )
  .fetch_all(exec)
  .await
}
