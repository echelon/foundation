//! `chat_gpt_5_4_nano` — gpt-5.4-nano via `/v1/responses` (`background: true`, then poll with
//! `get_response` / `await_response`). Nothing is held open.

use crate::api::requests::chat::common::{start_chat, ChatKnobs};
use crate::api::requests::responses::create_response::StartedResponse;
use crate::api::types::chat_types::chat_message::ChatMessage;
use crate::api::types::chat_types::chat_model::ChatModel;
use crate::creds::openai_api_key::OpenAiApiKey;
use crate::error::openai_error::OpenAiError;

// ── Public args ──

#[derive(Clone, Debug)]
pub struct ChatGpt5_4NanoArgs<'a> {
  pub api_key: &'a OpenAiApiKey,
  pub request: ChatGpt5_4NanoRequest,
}

/// One gpt-5.4-nano turn. The model is pinned by this binding.
#[derive(Clone, Debug)]
pub struct ChatGpt5_4NanoRequest {
  pub messages: Vec<ChatMessage>,
  pub max_output_tokens: Option<u32>,
  pub temperature: Option<f32>,
  pub top_p: Option<f32>,
  pub user: Option<String>,
}

impl ChatGpt5_4NanoRequest {
  pub fn new(messages: Vec<ChatMessage>) -> Self {
    Self { messages, max_output_tokens: None, temperature: None, top_p: None, user: None }
  }
}

pub(crate) fn knobs(request: &ChatGpt5_4NanoRequest) -> ChatKnobs<'_> {
  ChatKnobs {
    messages: &request.messages,
    max_output_tokens: request.max_output_tokens,
    temperature: request.temperature,
    top_p: request.top_p,
    user: request.user.as_deref(),
  }
}

/// Start a gpt-5.4-nano completion in the background; poll for the text.
pub async fn chat_gpt_5_4_nano(args: ChatGpt5_4NanoArgs<'_>) -> Result<StartedResponse, OpenAiError> {
  start_chat(args.api_key, &ChatModel::Gpt5_4Nano, &knobs(&args.request)).await
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::api::requests::chat::common::build_chat_body;
  use crate::error::openai_client_error::OpenAiClientError;

  /// Input struct → exact outbound HTTP body shape.
  #[test]
  fn wire_shape() {
    let mut req = ChatGpt5_4NanoRequest::new(vec![ChatMessage::system("be terse"), ChatMessage::user("hi")]);
    req.max_output_tokens = Some(128);
    let body = serde_json::to_value(build_chat_body(&ChatModel::Gpt5_4Nano, &knobs(&req)).unwrap()).unwrap();
    assert_eq!(body["model"], "gpt-5.4-nano");
    assert_eq!(body["background"], true);
    assert_eq!(body["store"], true);
    assert_eq!(body["max_output_tokens"], 128);
    assert_eq!(body["input"][0]["role"], "system");
    assert_eq!(body["input"][0]["content"], "be terse");
    assert_eq!(body["input"][1]["role"], "user");
    assert_eq!(body["input"][1]["content"], "hi");
    assert!(body.get("temperature").is_none());
    assert!(body.get("tools").is_none());
  }

  #[test]
  fn empty_messages_rejected() {
    let req = ChatGpt5_4NanoRequest::new(vec![]);
    assert!(matches!(
      build_chat_body(&ChatModel::Gpt5_4Nano, &knobs(&req)),
      Err(OpenAiClientError::InvalidRequest(_))
    ));
  }

  #[tokio::test]
  #[ignore] // live — costs money; needs external/credentials/openai_api_key.txt
  async fn live_chat_gpt_5_4_nano() -> anyhow::Result<()> {
    use crate::api::requests::responses::await_response::{await_response, AwaitResponseArgs, PollConfig};
    use crate::api::types::response_types::response_status::ResponseStatus;
    use crate::test_utils::get_test_api_key::get_test_api_key;
    use crate::test_utils::setup_test_logging::setup_test_logging;
    setup_test_logging();
    let key = get_test_api_key()?;
    let mut req = ChatGpt5_4NanoRequest::new(vec![ChatMessage::user("Reply with exactly one word: hello")]);
    req.max_output_tokens = Some(2048);
    let started = chat_gpt_5_4_nano(ChatGpt5_4NanoArgs { api_key: &key, request: req }).await.map_err(|e| anyhow::anyhow!("{e}"))?;
    let done = await_response(
      AwaitResponseArgs { api_key: &key, response_id: &started.response_id },
      PollConfig::default(),
    )
    .await
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    anyhow::ensure!(done.status == ResponseStatus::Completed, "status: {:?} ({:?})", done.status, done.failure);
    anyhow::ensure!(done.output_text.as_deref().is_some_and(|t| !t.is_empty()));
    Ok(())
  }
}
