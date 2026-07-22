//! `POST /v1/webhooks/openai` — the inbound door for OpenAI's project webhooks
//! (Standard Webhooks signatures; `response.completed` / `.failed` / `.cancelled` /
//! `.incomplete` for our background responses).
//!
//! Every delivery is logged to `inbound_webhooks` — verified or not — then correlated
//! back to its originating outbound request: the event's `data.id` IS the `resp_…` handle
//! the create call returned, so we look it up in `provider_calls.provider_job_id` (giving
//! the request_id) and in the job/slice tables. A matched terminal event triggers the SAME
//! fetch-and-ingest path the poll loop runs — a webhook is just an earlier poll trigger,
//! and the poll loop remains the safety net if processing here fails.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use db_queries::inbound_webhooks::{self, NewInboundWebhook};
use db_queries::{job_slices, jobs, provider_calls};
use enums::InboundWebhookOutcome;
use openai_client::api::webhooks::{parse_verified_webhook, VerifyWebhookArgs};

use crate::error::{ApiError, ApiResult};
use crate::generation::dispatcher::{poll_and_finish_slice, poll_and_finish_unsliced_job};
use crate::state::AppState;

const PROVIDER: &str = "openai";

#[derive(Debug, Serialize, ToSchema)]
pub struct WebhookAck {
  pub ok: bool,
  /// What the gateway did with the delivery: `processed`, `unmatched`, `duplicate`,
  /// `ignored`, or `error` (logged; the poll loop will finish the job).
  pub outcome: String,
}

/// Receives OpenAI's signed webhook deliveries. Headers `webhook-id`, `webhook-timestamp`,
/// and `webhook-signature` carry the Standard Webhooks signature over the RAW body —
/// verified before anything is trusted. Every delivery (verified or not) is recorded in
/// the inbound-webhook log with its correlation ids when they can be found.
#[utoipa::path(
  post, path = "/v1/webhooks/openai",
  tag = "webhooks",
  request_body(content = String, content_type = "application/json",
    description = "The raw OpenAI event payload — must be byte-identical to what was signed"),
  responses(
    (status = 200, description = "Delivery accepted (logged; processed when it matches in-flight work)", body = WebhookAck),
    (status = 401, description = "Signature verification failed — delivery logged and rejected"),
    (status = 500, description = "OPENAI_WEBHOOK_SECRET is not configured on this server")
  )
)]
pub async fn openai_webhook_handler(
  State(state): State<AppState>,
  headers: HeaderMap,
  body: Bytes,
) -> ApiResult<Json<WebhookAck>> {
  let body_str = String::from_utf8_lossy(&body);
  let payload_json: Option<serde_json::Value> = serde_json::from_slice(&body).ok();

  let header = |name: &str| headers.get(name).and_then(|v| v.to_str().ok()).unwrap_or("");
  let webhook_id = header("webhook-id");
  let webhook_timestamp = header("webhook-timestamp");
  let webhook_signature = header("webhook-signature");

  // No secret = nothing can be verified. Log the arrival, refuse loudly (OpenAI will
  // retry; deliveries are not silently dropped while misconfigured).
  let Some(secret) = state.openai_webhook_secret.clone() else {
    log_delivery(&state, LogDelivery::unverified(InboundWebhookOutcome::NotConfigured, payload_json.as_ref())).await?;
    return Err(ApiError::Internal("OPENAI_WEBHOOK_SECRET is not configured".into()));
  };

  // Verify FIRST — nothing in the payload is trusted until the signature checks out.
  let event = match parse_verified_webhook(&VerifyWebhookArgs {
    secret: &secret,
    webhook_id,
    webhook_timestamp,
    webhook_signature,
    body: &body_str,
  }) {
    Ok(event) => event,
    Err(e) => {
      tracing::warn!("openai webhook rejected: {e}");
      log_delivery(&state, LogDelivery::unverified(InboundWebhookOutcome::SignatureFailed, payload_json.as_ref()))
        .await?;
      return Err(ApiError::Unauthorized("webhook signature verification failed".into()));
    }
  };

  // Correlate: the subject id is the resp_… handle our create call received. The
  // provider_calls row (if found) carries the request_id for the debug view; the
  // slice/job rows are what processing advances.
  let object_id = event.object_id.clone();
  let pcall = provider_calls::find_provider_call_by_provider_job_id(&state.pool, PROVIDER, &object_id).await?;
  let slice = job_slices::find_slice_by_provider_job_id(&state.pool, &object_id).await?;
  let job = match &slice {
    Some(s) => jobs::find_job(&state.pool, &s.job_id).await?,
    None => jobs::find_job_by_provider_job_id(&state.pool, PROVIDER, &object_id).await?,
  };

  let mut log = LogDelivery {
    event_id: Some(event.event_id.clone()),
    event_type: Some(wire_event_type(&event)),
    object_id: Some(object_id.clone()),
    is_signature_valid: true,
    provider_call_id: pcall.as_ref().map(|c| c.id.clone()),
    job_id: job.as_ref().map(|j| j.id.clone()),
    job_slice_id: slice.as_ref().map(|s| s.id.clone()),
    outcome: InboundWebhookOutcome::Processed,
    payload: payload_json,
  };

  // At-least-once delivery: a redelivery of an event we already handled is logged as a
  // duplicate and acked without reprocessing.
  if inbound_webhooks::exists_verified_inbound_event(&state.pool, PROVIDER, &event.event_id).await? {
    log.outcome = InboundWebhookOutcome::Duplicate;
    log_delivery(&state, log).await?;
    return Ok(Json(WebhookAck { ok: true, outcome: "duplicate".into() }));
  }

  log.outcome = if !event.kind.is_response_event() {
    InboundWebhookOutcome::Ignored
  } else if slice.as_ref().is_some_and(|s| s.status.is_terminal())
    || (slice.is_none() && job.as_ref().is_some_and(|j| j.status.is_terminal()))
  {
    // Late/duplicate delivery for work that already finished (usually the poll loop won
    // the race) — re-ingesting would duplicate outputs, so just acknowledge.
    InboundWebhookOutcome::Ignored
  } else if let Some(slice) = &slice {
    match poll_and_finish_slice(&state, slice).await {
      Ok(()) => InboundWebhookOutcome::Processed,
      Err(e) => {
        tracing::error!("webhook-triggered slice processing failed for {} ({e}) — poll loop will retry", slice.id);
        InboundWebhookOutcome::Error
      }
    }
  } else if let Some(job) = &job {
    match poll_and_finish_unsliced_job(&state, job).await {
      Ok(()) => InboundWebhookOutcome::Processed,
      Err(e) => {
        tracing::error!("webhook-triggered job processing failed for {} ({e}) — poll loop will retry", job.id);
        InboundWebhookOutcome::Error
      }
    }
  } else {
    InboundWebhookOutcome::Unmatched
  };

  let outcome = log.outcome;
  log_delivery(&state, log).await?;
  Ok(Json(WebhookAck { ok: true, outcome: outcome.as_str().to_string() }))
}

/// The wire `type` string for the log (round-trips Unknown values verbatim).
fn wire_event_type(event: &openai_client::api::webhooks::WebhookEvent) -> String {
  use openai_client::api::webhooks::WebhookEventKind;
  match &event.kind {
    WebhookEventKind::ResponseCompleted => "response.completed".to_string(),
    WebhookEventKind::ResponseFailed => "response.failed".to_string(),
    WebhookEventKind::ResponseCancelled => "response.cancelled".to_string(),
    WebhookEventKind::ResponseIncomplete => "response.incomplete".to_string(),
    WebhookEventKind::Unknown(other) => other.clone(),
  }
}

/// Owned insert args for the log row (the handler builds it up as correlation resolves).
struct LogDelivery {
  event_id: Option<String>,
  event_type: Option<String>,
  object_id: Option<String>,
  is_signature_valid: bool,
  provider_call_id: Option<ids::ProviderCallId>,
  job_id: Option<ids::JobId>,
  job_slice_id: Option<ids::JobSliceId>,
  outcome: InboundWebhookOutcome,
  payload: Option<serde_json::Value>,
}

impl LogDelivery {
  /// A delivery rejected before (or without) verification. The identifying fields are
  /// pulled from the UNVERIFIED payload — untrusted, but exactly what an operator needs
  /// to line a rejected delivery up against the sender's logs (`is_signature_valid`
  /// stays false; nothing here feeds correlation or processing).
  fn unverified(outcome: InboundWebhookOutcome, payload: Option<&serde_json::Value>) -> Self {
    let claimed = |path: &[&str]| -> Option<String> {
      let mut v = payload?;
      for key in path {
        v = v.get(key)?;
      }
      v.as_str().map(str::to_string)
    };
    Self {
      event_id: claimed(&["id"]),
      event_type: claimed(&["type"]),
      object_id: claimed(&["data", "id"]),
      is_signature_valid: false,
      provider_call_id: None,
      job_id: None,
      job_slice_id: None,
      outcome,
      payload: payload.cloned(),
    }
  }
}

async fn log_delivery(state: &AppState, log: LogDelivery) -> ApiResult<()> {
  inbound_webhooks::create_inbound_webhook(
    &state.pool,
    NewInboundWebhook {
      provider: PROVIDER,
      event_id: log.event_id.as_deref(),
      event_type: log.event_type.as_deref(),
      object_id: log.object_id.as_deref(),
      is_signature_valid: log.is_signature_valid,
      provider_call_id: log.provider_call_id.as_ref(),
      job_id: log.job_id.as_ref(),
      job_slice_id: log.job_slice_id.as_ref(),
      outcome: log.outcome,
      payload: log.payload.as_ref(),
    },
  )
  .await?;
  Ok(())
}
