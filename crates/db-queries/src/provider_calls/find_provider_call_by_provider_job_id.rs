use sqlx::PgExecutor;

use super::ProviderCall;

/// The outbound request that produced this provider handle — how an inbound webhook finds
/// its request_id. Latest row wins if the handle ever repeats.
pub async fn find_provider_call_by_provider_job_id<'e, E: PgExecutor<'e>>(
  exec: E,
  provider: &str,
  provider_job_id: &str,
) -> sqlx::Result<Option<ProviderCall>> {
  sqlx::query_as!(
    ProviderCall,
    r#"SELECT id AS "id: ids::ProviderCallId", job_id AS "job_id: ids::JobId",
                 job_slice_id AS "job_slice_id?: ids::JobSliceId", provider, direction, is_dry_run,
                 provider_job_id, url, request_body, response_status, response_body, latency_ms,
                 created_at
       FROM provider_calls
       WHERE provider = $1 AND provider_job_id = $2
       ORDER BY sid DESC
       LIMIT 1"#,
    provider,
    provider_job_id,
  )
  .fetch_optional(exec)
  .await
}
