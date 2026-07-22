//! `chat_gpt_5_5` — gpt-5.5 via `/v1/responses` (`background: true`, then poll with
//! `get_response` / `await_response`). Nothing is held open.

use crate::api::requests::chat::common::{start_chat, ChatKnobs};
use crate::api::requests::responses::create_response::StartedResponse;
use crate::api::types::chat_types::chat_message::ChatMessage;
use crate::api::types::chat_types::chat_model::ChatModel;
use crate::creds::openai_api_key::OpenAiApiKey;
use crate::error::openai_error::OpenAiError;

// ── Public args ──

#[derive(Clone, Debug)]
pub struct ChatGpt5_5Args<'a> {
  pub api_key: &'a OpenAiApiKey,
  pub request: ChatGpt5_5Request,
}

/// One gpt-5.5 turn. The model is pinned by this binding.
#[derive(Clone, Debug)]
pub struct ChatGpt5_5Request {
  pub messages: Vec<ChatMessage>,
  pub max_output_tokens: Option<u32>,
  pub temperature: Option<f32>,
  pub top_p: Option<f32>,
  pub user: Option<String>,
}

impl ChatGpt5_5Request {
  pub fn new(messages: Vec<ChatMessage>) -> Self {
    Self { messages, max_output_tokens: None, temperature: None, top_p: None, user: None }
  }
}

pub(crate) fn knobs(request: &ChatGpt5_5Request) -> ChatKnobs<'_> {
  ChatKnobs {
    messages: &request.messages,
    max_output_tokens: request.max_output_tokens,
    temperature: request.temperature,
    top_p: request.top_p,
    user: request.user.as_deref(),
  }
}

/// Start a gpt-5.5 completion in the background; poll for the text.
pub async fn chat_gpt_5_5(args: ChatGpt5_5Args<'_>) -> Result<StartedResponse, OpenAiError> {
  start_chat(args.api_key, &ChatModel::Gpt5_5, &knobs(&args.request)).await
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::api::requests::chat::common::build_chat_body;
  use crate::error::openai_client_error::OpenAiClientError;

  /// Input struct → exact outbound HTTP body shape.
  #[test]
  fn wire_shape() {
    let mut req = ChatGpt5_5Request::new(vec![ChatMessage::system("be terse"), ChatMessage::user("hi")]);
    req.max_output_tokens = Some(128);
    let body = serde_json::to_value(build_chat_body(&ChatModel::Gpt5_5, &knobs(&req)).unwrap()).unwrap();
    assert_eq!(body["model"], "gpt-5.5");
    assert_eq!(body["background"], true);
    assert_eq!(body["store"], true);
    assert_eq!(body["max_output_tokens"], 128);
    assert_eq!(body["input"][0]["role"], "system");
    assert_eq!(body["input"][0]["content"], "be terse");
    assert_eq!(body["input"][1]["role"], "user");
    assert_eq!(body["input"][1]["content"], "hi");
    assert!(body.get("temperature").is_none());
    assert!(body.get("tools").is_none());
    assert!(body.get("metadata").is_none()); // correlation is by response id, not metadata
  }

  #[test]
  fn empty_messages_rejected() {
    let req = ChatGpt5_5Request::new(vec![]);
    assert!(matches!(
      build_chat_body(&ChatModel::Gpt5_5, &knobs(&req)),
      Err(OpenAiClientError::InvalidRequest(_))
    ));
  }

  // ── Live tests — cost money. Run with:
  //    cargo test -p openai_client chat_gpt_5_5 -- --ignored --nocapture
  // Responses are PRINTED before each test ends (that's half the fun).

  async fn run_live(messages: Vec<ChatMessage>, max_output_tokens: u32) -> anyhow::Result<String> {
    use crate::api::requests::responses::await_response::{await_response, AwaitResponseArgs, PollConfig};
    use crate::api::types::response_types::response_status::ResponseStatus;
    use crate::test_utils::get_test_api_key::get_test_api_key;
    use crate::test_utils::setup_test_logging::setup_test_logging;
    setup_test_logging();
    let key = get_test_api_key()?;
    let mut req = ChatGpt5_5Request::new(messages);
    req.max_output_tokens = Some(max_output_tokens);
    let started = chat_gpt_5_5(ChatGpt5_5Args { api_key: &key, request: req }).await.map_err(|e| anyhow::anyhow!("{e}"))?;
    let done = await_response(
      AwaitResponseArgs { api_key: &key, response_id: &started.response_id },
      PollConfig::default(),
    )
    .await
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    anyhow::ensure!(done.status == ResponseStatus::Completed, "status: {:?} ({:?})", done.status, done.failure);
    let text = done.output_text.unwrap_or_default();
    anyhow::ensure!(!text.is_empty(), "empty response text");
    if let Some(u) = &done.usage {
      println!("\n── gpt-5.5 · {} in / {} out tokens", u.input_tokens, u.output_tokens);
    }
    println!("── gpt-5.5 says:\n{text}\n");
    Ok(text)
  }

  #[tokio::test]
  #[ignore] // live — costs money
  async fn live_chat_gpt_5_5() -> anyhow::Result<()> {
    let text = run_live(vec![ChatMessage::user("Reply with exactly one word: hello")], 2048).await?;
    anyhow::ensure!(text.to_lowercase().contains("hello"), "expected a hello, got: {text}");
    Ok(())
  }

  #[tokio::test]
  #[ignore] // live — costs money
  async fn live_gpt_5_5_explains_to_a_rubber_duck() -> anyhow::Result<()> {
    let text = run_live(
      vec![ChatMessage::user(
        "Explain eventual consistency to a rubber duck in exactly three sentences. \
         Address the duck directly.",
      )],
      4096,
    )
    .await?;
    anyhow::ensure!(text.to_lowercase().contains("duck") || text.to_lowercase().contains("quack"), "no duck? {text}");
    Ok(())
  }

  #[tokio::test]
  #[ignore] // live — costs money
  async fn live_gpt_5_5_writes_a_database_limerick() -> anyhow::Result<()> {
    let text = run_live(
      vec![ChatMessage::user("Write a limerick about a database that drops its own tables.")],
      4096,
    )
    .await?;
    anyhow::ensure!(text.lines().filter(|l| !l.trim().is_empty()).count() >= 5, "expected 5+ lines: {text}");
    Ok(())
  }
}
