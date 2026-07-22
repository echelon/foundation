use enums::InboundWebhookOutcome;
use ids::{InboundWebhookId, JobId, JobSliceId, ProviderCallId};
use sqlx::PgExecutor;

use super::InboundWebhook;

#[derive(Debug)]
pub struct NewInboundWebhook<'a> {
  pub provider: &'a str,
  pub event_id: Option<&'a str>,
  pub event_type: Option<&'a str>,
  pub object_id: Option<&'a str>,
  pub is_signature_valid: bool,
  pub provider_call_id: Option<&'a ProviderCallId>,
  pub job_id: Option<&'a JobId>,
  pub job_slice_id: Option<&'a JobSliceId>,
  pub outcome: InboundWebhookOutcome,
  /// The parsed event payload; None when the body wasn't valid JSON.
  pub payload: Option<&'a serde_json::Value>,
}

pub async fn create_inbound_webhook<'e, E: PgExecutor<'e>>(
  exec: E,
  new: NewInboundWebhook<'_>,
) -> sqlx::Result<InboundWebhook> {
  let id = InboundWebhookId::generate();
  sqlx::query_as!(
    InboundWebhook,
    r#"INSERT INTO inbound_webhooks
         (id, provider, event_id, event_type, object_id, is_signature_valid,
          provider_call_id, job_id, job_slice_id, outcome, payload)
       VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
       RETURNING id AS "id: ids::InboundWebhookId", provider, event_id, event_type, object_id,
                 is_signature_valid, provider_call_id AS "provider_call_id?: ids::ProviderCallId",
                 job_id AS "job_id?: ids::JobId", job_slice_id AS "job_slice_id?: ids::JobSliceId",
                 outcome AS "outcome: enums::InboundWebhookOutcome", payload, created_at"#,
    id.as_str(),
    new.provider,
    new.event_id,
    new.event_type,
    new.object_id,
    new.is_signature_valid,
    new.provider_call_id.map(|p| p.as_str()),
    new.job_id.map(|j| j.as_str()),
    new.job_slice_id.map(|s| s.as_str()),
    new.outcome.as_str(),
    new.payload,
  )
  .fetch_one(exec)
  .await
}
