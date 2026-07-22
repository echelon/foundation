//! `inbound_webhooks` table (logging.md addendum 2026-07-22) — one row per provider→us
//! webhook delivery, verified or not. Correlation ids are best-effort: a delivery we can't
//! match still gets logged (`outcome = unmatched`).

use chrono::{DateTime, Utc};
use enums::InboundWebhookOutcome;
use ids::{InboundWebhookId, JobId, JobSliceId, ProviderCallId};

mod create_inbound_webhook;
mod exists_verified_inbound_event;

pub use create_inbound_webhook::{create_inbound_webhook, NewInboundWebhook};
pub use exists_verified_inbound_event::exists_verified_inbound_event;

#[derive(Debug, Clone)]
pub struct InboundWebhook {
  pub id: InboundWebhookId,
  pub provider: String,
  pub event_id: Option<String>,
  pub event_type: Option<String>,
  pub object_id: Option<String>,
  pub is_signature_valid: bool,
  pub provider_call_id: Option<ProviderCallId>,
  pub job_id: Option<JobId>,
  pub job_slice_id: Option<JobSliceId>,
  pub outcome: InboundWebhookOutcome,
  pub payload: Option<serde_json::Value>,
  pub created_at: DateTime<Utc>,
}
