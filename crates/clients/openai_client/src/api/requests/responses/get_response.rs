//! `GET /v1/responses/{id}` — the poll. Keep calling while the status is non-terminal
//! (`queued` / `in_progress`); [`await_response`](fn@super::await_response::await_response) wraps the loop.

use base64::Engine;
use tracing::info;

use crate::api::requests::openai_host::{with_auth, OPENAI_API_BASE_URL};
use crate::api::requests::responses::wire::ResponseWire;
use crate::api::types::image_types::output_format::OutputFormat;
use crate::api::types::response_types::response_id::ResponseId;
use crate::api::types::response_types::response_status::ResponseStatus;
use crate::creds::openai_api_key::OpenAiApiKey;
use crate::error::classify_openai_http_error::classify_openai_http_error;
use crate::error::openai_error::OpenAiError;
use crate::error::openai_generic_api_error::OpenAiGenericApiError;

// ── Public args ──

#[derive(Debug)]
pub struct GetResponseArgs<'a> {
  pub api_key: &'a OpenAiApiKey,
  pub response_id: &'a ResponseId,
}

// ── Public result ──

/// The state of a background response at one poll: status plus whatever output exists so
/// far. On `Completed`, `output_text` (LLM) or `images` (image tool) carry the goods and
/// `usage` carries the AUTHORITATIVE token counts for billing.
#[derive(Debug)]
pub struct ResponseSnapshot {
  pub response_id: ResponseId,
  pub status: ResponseStatus,
  pub model: Option<String>,
  /// Assistant text output (LLM bindings), all message parts joined.
  pub output_text: Option<String>,
  /// Generated images (image bindings), decoded from base64.
  pub images: Vec<GeneratedImage>,
  pub usage: Option<ResponseUsage>,
  /// Populated when `status` is `Failed` (and sometimes `Incomplete`).
  pub failure: Option<ResponseFailure>,
}

pub struct GeneratedImage {
  pub bytes: Vec<u8>,
  pub format: OutputFormat,
}

// Debug elides the bytes.
impl std::fmt::Debug for GeneratedImage {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("GeneratedImage")
      .field("len", &self.bytes.len())
      .field("format", &self.format)
      .finish()
  }
}

/// Authoritative post-call token counts — this is what OpenAI actually bills from, unlike
/// the pre-call `CostCalculator` ESTIMATES.
#[derive(Debug, Clone, Default)]
pub struct ResponseUsage {
  pub input_tokens: i64,
  pub output_tokens: i64,
  pub total_tokens: i64,
}

#[derive(Debug, Clone)]
pub struct ResponseFailure {
  pub code: Option<String>,
  pub message: String,
}

// ── Implementation ──

pub async fn get_response(args: GetResponseArgs<'_>) -> Result<ResponseSnapshot, OpenAiError> {
  let client = client_core::http_client("foundation/openai-client");
  let builder = client.get(format!("{OPENAI_API_BASE_URL}/v1/responses/{}", args.response_id.as_str()));

  let response = with_auth(builder, args.api_key)
    .send()
    .await
    .map_err(OpenAiGenericApiError::ReqwestError)?;

  let status = response.status();
  let text = response.text().await.map_err(OpenAiGenericApiError::ReqwestError)?;
  info!(%status, response_id = %args.response_id, "openai get response");

  classify_openai_http_error(status, Some(&text))?;

  let parsed: ResponseWire = serde_json::from_str(&text)
    .map_err(|e| OpenAiGenericApiError::SerdeResponseParseErrorWithBody(e, text.clone()))?;
  Ok(snapshot_from_wire(parsed))
}

pub(crate) fn snapshot_from_wire(wire: ResponseWire) -> ResponseSnapshot {
  let mut texts: Vec<String> = Vec::new();
  let mut images: Vec<GeneratedImage> = Vec::new();

  for item in &wire.output {
    match item.kind.as_str() {
      "message" => {
        for part in &item.content {
          if part.kind == "output_text" {
            if let Some(t) = &part.text {
              texts.push(t.clone());
            }
          }
        }
      }
      "image_generation_call" => {
        if let Some(b64) = &item.result {
          if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(b64) {
            let format = OutputFormat::from_wire(item.output_format.as_deref().unwrap_or("png"));
            images.push(GeneratedImage { bytes, format });
          }
        }
      }
      _ => {} // reasoning, tool traces, future item types — irrelevant here
    }
  }

  let failure = match (&wire.error, &wire.incomplete_details) {
    (Some(e), _) => Some(ResponseFailure {
      code: e.code.clone(),
      message: e.message.clone().unwrap_or_else(|| "response failed".into()),
    }),
    (None, Some(inc)) => inc.reason.as_ref().map(|r| ResponseFailure {
      code: Some("incomplete".into()),
      message: format!("response incomplete: {r}"),
    }),
    _ => None,
  };

  ResponseSnapshot {
    response_id: ResponseId(wire.id),
    status: ResponseStatus::from_wire(&wire.status),
    model: wire.model,
    output_text: if texts.is_empty() { None } else { Some(texts.join("")) },
    images,
    usage: wire.usage.map(|u| ResponseUsage {
      input_tokens: u.input_tokens,
      output_tokens: u.output_tokens,
      total_tokens: u.total_tokens,
    }),
    failure,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn snapshot_extracts_text_and_images() {
    let wire: ResponseWire = serde_json::from_str(
      r#"{
        "id": "resp_9", "status": "completed", "model": "gpt-5.5",
        "output": [
          { "type": "message", "content": [ { "type": "output_text", "text": "done" } ] },
          { "type": "image_generation_call", "result": "aGVsbG8=", "output_format": "webp" }
        ],
        "usage": { "input_tokens": 5, "output_tokens": 1100, "total_tokens": 1105 }
      }"#,
    )
    .unwrap();
    let snap = snapshot_from_wire(wire);
    assert_eq!(snap.status, ResponseStatus::Completed);
    assert_eq!(snap.output_text.as_deref(), Some("done"));
    assert_eq!(snap.images.len(), 1);
    assert_eq!(snap.images[0].bytes, b"hello");
    assert_eq!(snap.images[0].format, OutputFormat::Webp);
    assert_eq!(snap.usage.unwrap().output_tokens, 1100);
  }

  #[test]
  fn snapshot_surfaces_failures() {
    let wire: ResponseWire = serde_json::from_str(
      r#"{ "id": "resp_x", "status": "failed", "error": { "code": "server_error", "message": "boom" } }"#,
    )
    .unwrap();
    let snap = snapshot_from_wire(wire);
    assert_eq!(snap.status, ResponseStatus::Failed);
    assert_eq!(snap.failure.unwrap().message, "boom");
  }
}
