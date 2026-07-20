use serde::Serialize;
use tracing::info;

use crate::api::requests::chat::chat_completion::request_types::*;
use crate::api::requests::openai_host::{with_auth, OPENAI_API_BASE_URL};
use crate::api::types::chat_types::chat_message::ChatMessage;
use crate::api::types::chat_types::chat_model::ChatModel;
use crate::creds::openai_api_key::OpenAiApiKey;
use crate::error::classify_openai_http_error::classify_openai_http_error;
use crate::error::openai_client_error::OpenAiClientError;
use crate::error::openai_error::OpenAiError;
use crate::error::openai_generic_api_error::OpenAiGenericApiError;

// ── Public args ──

#[derive(Clone, Debug)]
pub struct ChatCompletionArgs<'a> {
  pub api_key: &'a OpenAiApiKey,
  pub request: ChatCompletionRequest,
}

/// A chat/LLM completion request. Serializes cleanly (no key) for logging/audit.
#[derive(Clone, Debug, Serialize)]
pub struct ChatCompletionRequest {
  pub model: ChatModel,
  pub messages: Vec<ChatMessage>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub max_output_tokens: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub temperature: Option<f32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub top_p: Option<f32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub user: Option<String>,
}

impl ChatCompletionRequest {
  pub fn new(model: ChatModel, messages: Vec<ChatMessage>) -> Self {
    Self { model, messages, max_output_tokens: None, temperature: None, top_p: None, user: None }
  }
}

// ── Public response ──

#[derive(Debug, Clone)]
pub struct ChatCompletionSuccess {
  pub id: String,
  pub model: String,
  /// The assistant's reply text (first choice).
  pub content: String,
  pub finish_reason: Option<String>,
  pub usage: ChatUsageWire,
}

// ── Implementation ──

/// `POST https://api.openai.com/v1/chat/completions` — an LLM chat completion.
///
/// Docs: <https://developers.openai.com/api/reference/resources/chat>
pub async fn chat_completion(args: ChatCompletionArgs<'_>) -> Result<ChatCompletionSuccess, OpenAiError> {
  let req = args.request;

  if req.messages.is_empty() {
    return Err(OpenAiClientError::InvalidRequest("messages must not be empty".into()).into());
  }

  info!(model = req.model.as_str(), messages = req.messages.len(), "openai chat_completion");

  let body = ChatCompletionRequestBody {
    model: req.model.as_str().to_string(),
    messages: req.messages,
    max_completion_tokens: req.max_output_tokens,
    temperature: req.temperature,
    top_p: req.top_p,
    user: req.user,
  };

  let client = client_core::http_client("foundation/openai-client");
  let builder = client
    .post(format!("{OPENAI_API_BASE_URL}/v1/chat/completions"))
    .header("Content-Type", "application/json")
    .json(&body);

  let response = with_auth(builder, args.api_key)
    .send()
    .await
    .map_err(OpenAiGenericApiError::ReqwestError)?;

  let status = response.status();
  let text = response.text().await.map_err(OpenAiGenericApiError::ReqwestError)?;
  info!(%status, "openai chat_completion response");

  classify_openai_http_error(status, Some(&text))?;

  let parsed: ChatCompletionResponseBody = serde_json::from_str(&text)
    .map_err(|e| OpenAiGenericApiError::SerdeResponseParseErrorWithBody(e, text.clone()))?;

  let first = parsed.choices.into_iter().next().ok_or_else(|| {
    OpenAiGenericApiError::UncategorizedBadResponseWithStatusAndBody {
      status_code: status,
      body: "chat response had no choices".into(),
    }
  })?;

  Ok(ChatCompletionSuccess {
    id: parsed.id,
    model: parsed.model,
    content: first.message.content.unwrap_or_default(),
    finish_reason: first.finish_reason,
    usage: parsed.usage.unwrap_or_default(),
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn request_serializes_no_key() {
    let req = ChatCompletionRequest {
      model: ChatModel::Gpt5_5,
      messages: vec![ChatMessage::user("hi")],
      max_output_tokens: Some(256),
      temperature: Some(0.7),
      top_p: None,
      user: None,
    };
    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("\"model\":\"gpt-5.5\""));
    assert!(json.contains("\"role\":\"user\""));
    assert!(json.contains("\"max_output_tokens\":256"));
    assert!(!json.contains("\"top_p\""));
  }

  #[tokio::test]
  async fn empty_messages_rejected() {
    let key = OpenAiApiKey::new("sk-test");
    let err = chat_completion(ChatCompletionArgs {
      api_key: &key,
      request: ChatCompletionRequest::new(ChatModel::Gpt5, vec![]),
    })
    .await
    .unwrap_err();
    assert!(matches!(err, OpenAiError::Client(OpenAiClientError::InvalidRequest(_))));
  }

  #[tokio::test]
  #[ignore] // live — requires OPENAI_API_KEY
  async fn live_chat() -> anyhow::Result<()> {
    use crate::test_utils::get_test_api_key::get_test_api_key;
    use crate::test_utils::setup_test_logging::setup_test_logging;
    setup_test_logging();
    let key = get_test_api_key()?;
    let res = chat_completion(ChatCompletionArgs {
      api_key: &key,
      request: ChatCompletionRequest {
        model: ChatModel::Gpt5Mini,
        messages: vec![ChatMessage::user("Say hello in one word.")],
        max_output_tokens: Some(16),
        temperature: None,
        top_p: None,
        user: None,
      },
    })
    .await
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    assert!(!res.content.is_empty());
    Ok(())
  }
}
