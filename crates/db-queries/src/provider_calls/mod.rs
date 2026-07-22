//! `provider_calls` table (logging.md §3) — the outbound third-party request log. One row
//! per provider round trip; `id` IS the request_id, minted BEFORE the send so failures
//! still capture the exact payload. Inbound webhooks correlate back through
//! `provider_job_id` (the provider's handle, e.g. an OpenAI `resp_…`).

use chrono::{DateTime, Utc};
use ids::{JobId, JobSliceId, ProviderCallId};

mod create_provider_call;
mod find_provider_call_by_provider_job_id;
mod list_provider_calls_for_job;
mod set_provider_call_result;

pub use create_provider_call::{create_provider_call, NewProviderCall};
pub use find_provider_call_by_provider_job_id::find_provider_call_by_provider_job_id;
pub use list_provider_calls_for_job::list_provider_calls_for_job;
pub use set_provider_call_result::{set_provider_call_result, ProviderCallResult};

/// A row from `provider_calls` — the columns this iteration reads.
#[derive(Debug, Clone)]
pub struct ProviderCall {
  pub id: ProviderCallId,
  pub job_id: JobId,
  pub job_slice_id: Option<JobSliceId>,
  pub provider: String,
  pub direction: String,
  pub is_dry_run: bool,
  pub provider_job_id: Option<String>,
  pub url: Option<String>,
  pub request_body: Option<serde_json::Value>,
  pub response_status: Option<i32>,
  pub response_body: Option<serde_json::Value>,
  pub latency_ms: Option<i32>,
  pub created_at: DateTime<Utc>,
}
