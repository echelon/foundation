use serde::Serialize;
use tracing::info;

use crate::api::requests::anthropic_host::{ANTHROPIC_API_BASE_URL, ANTHROPIC_VERSION};
use crate::api::requests::messages::create_message::request_types::*;
use crate::api::types::chat_types::anthropic_model::AnthropicModel;
use crate::api::types::chat_types::message::Message;
use crate::creds::anthropic_api_key::AnthropicApiKey;
use crate::error::anthropic_client_error::AnthropicClientError;
use crate::error::anthropic_error::AnthropicError;
use crate::error::anthropic_generic_api_error::AnthropicGenericApiError;
use crate::error::classify_anthropic_http_error::classify_anthropic_http_error;

// ── Public args ──

#[derive(Clone, Debug)]
pub struct CreateMessageArgs<'a> {
  pub api_key: &'a AnthropicApiKey,
  pub request: CreateMessageRequest,
}

/// A Messages API request. `max_tokens` is REQUIRED by Anthropic. Serializes without the key.
#[derive(Clone, Debug, Serialize)]
pub struct CreateMessageRequest {
  pub model: AnthropicModel,
  pub messages: Vec<Message>,
  pub max_tokens: u32,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub system: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub temperature: Option<f32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub top_p: Option<f32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub stop_sequences: Option<Vec<String>>,
}

impl CreateMessageRequest {
  pub fn new(model: AnthropicModel, messages: Vec<Message>, max_tokens: u32) -> Self {
    Self { model, messages, max_tokens, system: None, temperature: None, top_p: None, stop_sequences: None }
  }

  pub fn with_system(mut self, system: impl Into<String>) -> Self {
    self.system = Some(system.into());
    self
  }
}

// ── Public response ──

#[derive(Debug, Clone)]
pub struct CreateMessageSuccess {
  pub id: String,
  pub model: String,
  /// Concatenated text of all text content blocks.
  pub text: String,
  pub stop_reason: Option<String>,
  pub usage: MessageUsageWire,
}

// ── Implementation ──

/// `POST https://api.anthropic.com/v1/messages` — a Claude chat completion.
///
/// Docs: <https://docs.anthropic.com/en/api/messages>
pub async fn create_message(args: CreateMessageArgs<'_>) -> Result<CreateMessageSuccess, AnthropicError> {
  let req = args.request;

  if req.messages.is_empty() {
    return Err(AnthropicClientError::InvalidRequest("messages must not be empty".into()).into());
  }
  if req.max_tokens == 0 {
    return Err(AnthropicClientError::InvalidRequest("max_tokens must be >= 1".into()).into());
  }

  info!(model = req.model.as_str(), max_tokens = req.max_tokens, "anthropic create_message");

  let body = CreateMessageRequestBody {
    model: req.model.as_str().to_string(),
    max_tokens: req.max_tokens,
    messages: req.messages,
    system: req.system,
    temperature: req.temperature,
    top_p: req.top_p,
    stop_sequences: req.stop_sequences,
  };

  let client = client_core::http_client("foundation/anthropic-client");
  let response = client
    .post(format!("{ANTHROPIC_API_BASE_URL}/v1/messages"))
    .header("x-api-key", &args.api_key.api_key)
    .header("anthropic-version", ANTHROPIC_VERSION)
    .header("Content-Type", "application/json")
    .json(&body)
    .send()
    .await
    .map_err(AnthropicGenericApiError::ReqwestError)?;

  let status = response.status();
  let text = response.text().await.map_err(AnthropicGenericApiError::ReqwestError)?;
  info!(%status, "anthropic create_message response");

  classify_anthropic_http_error(status, Some(&text))?;

  let parsed: CreateMessageResponseBody = serde_json::from_str(&text)
    .map_err(|e| AnthropicGenericApiError::SerdeResponseParseErrorWithBody(e, text.clone()))?;

  let content: String = parsed.content.iter().filter_map(|b| b.text.clone()).collect();

  Ok(CreateMessageSuccess {
    id: parsed.id,
    model: parsed.model,
    text: content,
    stop_reason: parsed.stop_reason,
    usage: parsed.usage.unwrap_or_default(),
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn request_serializes_no_key() {
    let req = CreateMessageRequest::new(
      AnthropicModel::ClaudeOpus4_8,
      vec![Message::user("hi")],
      1024,
    )
    .with_system("You are terse.");
    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("\"model\":\"claude-opus-4-8\""));
    assert!(json.contains("\"max_tokens\":1024"));
    assert!(json.contains("\"system\":\"You are terse.\""));
    assert!(json.contains("\"role\":\"user\""));
    assert!(!json.contains("\"temperature\""));
  }

  #[tokio::test]
  async fn empty_messages_rejected() {
    let key = AnthropicApiKey::new("sk-ant-test");
    let err = create_message(CreateMessageArgs {
      api_key: &key,
      request: CreateMessageRequest::new(AnthropicModel::ClaudeHaiku4_5, vec![], 100),
    })
    .await
    .unwrap_err();
    assert!(matches!(err, AnthropicError::Client(AnthropicClientError::InvalidRequest(_))));
  }

  #[tokio::test]
  #[ignore] // live — requires ANTHROPIC_API_KEY
  async fn live_create_message() -> anyhow::Result<()> {
    use crate::test_utils::get_test_api_key::get_test_api_key;
    use crate::test_utils::setup_test_logging::setup_test_logging;
    setup_test_logging();
    let key = get_test_api_key()?;
    let res = create_message(CreateMessageArgs {
      api_key: &key,
      request: CreateMessageRequest::new(
        AnthropicModel::ClaudeHaiku4_5,
        vec![Message::user("Say hello in one word.")],
        32,
      ),
    })
    .await
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    assert!(!res.text.is_empty());
    Ok(())
  }
}
