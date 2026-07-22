//! Inbound webhook event parsing. OpenAI webhooks are registered per PROJECT (dashboard →
//! webhook settings) and fire for every background response. The event carries only ids —
//! `data.id` is the SAME `resp_…` id the create call returned, so correlation is: store the
//! id at start, match it here, then fetch the payload with `get_response`.

use serde::Deserialize;

use crate::api::types::response_types::response_id::ResponseId;

/// The event types we care about (background responses). Anything else — batch events,
/// fine-tuning, future types — lands in `Unknown` rather than failing the parse.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WebhookEventKind {
  ResponseCompleted,
  ResponseFailed,
  ResponseCancelled,
  ResponseIncomplete,
  Unknown(String),
}

impl WebhookEventKind {
  fn from_wire(s: &str) -> Self {
    match s {
      "response.completed" => Self::ResponseCompleted,
      "response.failed" => Self::ResponseFailed,
      "response.cancelled" => Self::ResponseCancelled,
      "response.incomplete" => Self::ResponseIncomplete,
      other => Self::Unknown(other.to_string()),
    }
  }

  /// Terminal response events — the ones worth fetching a payload for.
  pub fn is_response_event(&self) -> bool {
    !matches!(self, Self::Unknown(_))
  }
}

#[derive(Debug, Clone)]
pub struct WebhookEvent {
  /// The event id (`evt_…`) — dedupe key for at-least-once delivery.
  pub event_id: String,
  pub kind: WebhookEventKind,
  /// The subject object's id (a `resp_…` for response events).
  pub object_id: String,
  /// Unix seconds.
  pub created_at: i64,
}

impl WebhookEvent {
  /// The response id, when this event is about a background response.
  pub fn response_id(&self) -> Option<ResponseId> {
    if self.kind.is_response_event() || self.object_id.starts_with("resp_") {
      Some(ResponseId(self.object_id.clone()))
    } else {
      None
    }
  }
}

#[derive(Deserialize)]
struct EventWire {
  id: String,
  #[serde(rename = "type")]
  kind: String,
  #[serde(default)]
  created_at: i64,
  data: EventDataWire,
}

#[derive(Deserialize)]
struct EventDataWire {
  id: String,
}

/// Parse a webhook body. Verify the signature FIRST (`verify_webhook_signature`) — or use
/// `parse_verified_webhook`, which does both.
pub fn parse_webhook_event(body: &str) -> Result<WebhookEvent, serde_json::Error> {
  let wire: EventWire = serde_json::from_str(body)?;
  Ok(WebhookEvent {
    event_id: wire.id,
    kind: WebhookEventKind::from_wire(&wire.kind),
    object_id: wire.data.id,
    created_at: wire.created_at,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn response_completed_parses() {
    let event = parse_webhook_event(
      r#"{ "object": "event", "id": "evt_1", "type": "response.completed",
           "created_at": 1750287018, "data": { "id": "resp_abc123" } }"#,
    )
    .unwrap();
    assert_eq!(event.kind, WebhookEventKind::ResponseCompleted);
    assert_eq!(event.response_id().unwrap().as_str(), "resp_abc123");
    assert_eq!(event.created_at, 1750287018);
  }

  #[test]
  fn unknown_types_do_not_fail() {
    let event = parse_webhook_event(
      r#"{ "id": "evt_2", "type": "batch.completed", "created_at": 1, "data": { "id": "batch_9" } }"#,
    )
    .unwrap();
    assert_eq!(event.kind, WebhookEventKind::Unknown("batch.completed".into()));
    assert!(event.response_id().is_none());
  }
}
