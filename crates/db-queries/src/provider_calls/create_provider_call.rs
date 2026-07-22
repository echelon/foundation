use ids::{JobId, JobSliceId, ProviderCallId};
use sqlx::PgExecutor;

use super::ProviderCall;

#[derive(Debug)]
pub struct NewProviderCall<'a> {
  pub job_id: &'a JobId,
  /// Set when the call belongs to one fan-out slice.
  pub job_slice_id: Option<&'a JobSliceId>,
  pub provider: &'a str,
  pub url: &'a str,
  /// The EXACT provider-native request payload, serialized before send.
  pub request_body: &'a serde_json::Value,
  /// Dry run: the payload is logged but the request is never sent.
  pub is_dry_run: bool,
}

/// Insert the outbound-request row BEFORE sending (logging.md §1: failures still capture
/// the payload). The returned row's `id` is the request_id — hold it, update it with
/// [`super::set_provider_call_result`] after the send.
pub async fn create_provider_call<'e, E: PgExecutor<'e>>(
  exec: E,
  new: NewProviderCall<'_>,
) -> sqlx::Result<ProviderCall> {
  let id = ProviderCallId::generate();
  sqlx::query_as!(
    ProviderCall,
    r#"INSERT INTO provider_calls
         (id, job_id, job_slice_id, provider, direction, is_dry_run, url, request_body,
          is_request_truncated, is_response_truncated)
       VALUES ($1, $2, $3, $4, 'request', $5, $6, $7, false, false)
       RETURNING id AS "id: ids::ProviderCallId", job_id AS "job_id: ids::JobId",
                 job_slice_id AS "job_slice_id?: ids::JobSliceId", provider, direction, is_dry_run,
                 provider_job_id, url, request_body, response_status, response_body, latency_ms,
                 created_at"#,
    id.as_str(),
    new.job_id.as_str(),
    new.job_slice_id.map(|s| s.as_str()),
    new.provider,
    new.is_dry_run,
    new.url,
    new.request_body,
  )
  .fetch_one(exec)
  .await
}
