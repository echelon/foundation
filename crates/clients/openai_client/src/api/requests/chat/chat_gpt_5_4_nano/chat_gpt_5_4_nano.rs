//! `chat_gpt_5_4_nano` — gpt-5.4-nano via `/v1/responses` (`background: true`, then poll with
//! `get_response` / `await_response`). Nothing is held open.

use crate::api::requests::chat::common::{start_chat, ChatKnobs};
use crate::api::requests::responses::create_response::StartedResponse;
use crate::api::types::chat_types::chat_message::ChatMessage;
use crate::api::types::chat_types::chat_model::ChatModel;
use crate::api::types::chat_types::reasoning_effort::ReasoningEffort;

pub use crate::api::requests::chat::common::JsonSchemaFormat;
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
  /// The conversation so far (at least one message; system turns allowed).
  pub messages: Vec<ChatMessage>,
  /// Cap on generated tokens (also the cost estimator's output assumption).
  pub max_output_tokens: Option<u32>,
  /// Sampling temperature; server default when `None`.
  pub temperature: Option<f32>,
  /// Nucleus sampling; server default when `None`.
  pub top_p: Option<f32>,
  /// End-user identifier for abuse attribution.
  pub user: Option<String>,
  /// GPT-5-family reasoning depth (`reasoning.effort`); supported values are
  /// model-dependent — see [`ReasoningEffort`].
  pub reasoning_effort: Option<ReasoningEffort>,
  /// Structured output: constrain the reply to a JSON Schema
  /// (`text.format` json_schema).
  pub response_json_schema: Option<JsonSchemaFormat>,
  /// Continue a stored server-side conversation from a prior response id.
  pub previous_response_id: Option<String>,
}

impl ChatGpt5_4NanoRequest {
  pub fn new(messages: Vec<ChatMessage>) -> Self {
    Self {
      messages,
      max_output_tokens: None,
      temperature: None,
      top_p: None,
      user: None,
      reasoning_effort: None,
      response_json_schema: None,
      previous_response_id: None,
    }
  }
}

pub(crate) fn knobs(request: &ChatGpt5_4NanoRequest) -> ChatKnobs<'_> {
  ChatKnobs {
    messages: &request.messages,
    max_output_tokens: request.max_output_tokens,
    temperature: request.temperature,
    top_p: request.top_p,
    user: request.user.as_deref(),
    reasoning_effort: request.reasoning_effort,
    response_json_schema: request.response_json_schema.as_ref(),
    previous_response_id: request.previous_response_id.as_deref(),
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
    assert!(body.get("metadata").is_none()); // correlation is by response id, not metadata
  }

  /// The full-featured knob set → exact wire fields.
  #[test]
  fn wire_shape_with_reasoning_schema_and_continuation() {
    let mut req = ChatGpt5_4NanoRequest::new(vec![ChatMessage::user("extract the date")]);
    req.reasoning_effort = Some(ReasoningEffort::Low);
    req.previous_response_id = Some("resp_prior".to_string());
    req.response_json_schema = Some(JsonSchemaFormat {
      name: "date_extraction".to_string(),
      schema: client_core::JsonSchemaDef::new(serde_json::json!({
        "type": "object", "properties": { "date": { "type": "string" } },
        "required": ["date"], "additionalProperties": false
      }))
      .unwrap(),
      strict: true,
    });
    let body = serde_json::to_value(build_chat_body(&ChatModel::Gpt5_4Nano, &knobs(&req)).unwrap()).unwrap();
    assert_eq!(body["reasoning"]["effort"], "low");
    assert_eq!(body["previous_response_id"], "resp_prior");
    assert_eq!(body["text"]["format"]["type"], "json_schema");
    assert_eq!(body["text"]["format"]["name"], "date_extraction");
    assert_eq!(body["text"]["format"]["strict"], true);
    assert_eq!(body["text"]["format"]["schema"]["required"][0], "date");
  }

  #[test]
  fn empty_messages_rejected() {
    let req = ChatGpt5_4NanoRequest::new(vec![]);
    assert!(matches!(
      build_chat_body(&ChatModel::Gpt5_4Nano, &knobs(&req)),
      Err(OpenAiClientError::InvalidRequest(_))
    ));
  }

  // ── Live tests — cost money. Run with:
  //    cargo test -p openai_client chat_gpt_5_4_nano -- --ignored --nocapture
  // Responses are PRINTED before each test ends (that's half the fun).

  async fn run_live(messages: Vec<ChatMessage>, max_output_tokens: u32) -> anyhow::Result<String> {
    use crate::api::requests::responses::await_response::{await_response, AwaitResponseArgs, PollConfig};
    use crate::api::types::response_types::response_status::ResponseStatus;
    use crate::test_utils::get_test_api_key::get_test_api_key;
    use crate::test_utils::setup_test_logging::setup_test_logging;
    setup_test_logging();
    let key = get_test_api_key()?;
    let mut req = ChatGpt5_4NanoRequest::new(messages);
    req.max_output_tokens = Some(max_output_tokens);
    let started = chat_gpt_5_4_nano(ChatGpt5_4NanoArgs { api_key: &key, request: req }).await.map_err(|e| anyhow::anyhow!("{e}"))?;
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
      println!("\n── gpt-5.4-nano · {} in / {} out tokens", u.input_tokens, u.output_tokens);
    }
    println!("── gpt-5.4-nano says:\n{text}\n");
    Ok(text)
  }

  #[tokio::test]
  #[ignore] // live — costs money
  async fn live_chat_gpt_5_4_nano() -> anyhow::Result<()> {
    let text = run_live(vec![ChatMessage::user("Reply with exactly one word: hello")], 2048).await?;
    anyhow::ensure!(text.to_lowercase().contains("hello"), "expected a hello, got: {text}");
    Ok(())
  }

  #[tokio::test]
  #[ignore] // live — costs money
  async fn live_gpt_5_4_nano_tells_a_joke() -> anyhow::Result<()> {
    let text = run_live(
      vec![ChatMessage::user("Tell me one short, original joke about API rate limits.")],
      2048,
    )
    .await?;
    anyhow::ensure!(text.len() > 10, "suspiciously short joke: {text}");
    Ok(())
  }

  #[tokio::test]
  #[ignore] // live — costs money
  async fn live_gpt_5_4_nano_names_a_business() -> anyhow::Result<()> {
    // Fitting, given our placeholder-business-name generator.
    let text = run_live(
      vec![ChatMessage::user(
        "Invent five two-word fictional business names in the pattern 'Adjective Noun'. \
         One per line, no numbering, no commentary.",
      )],
      2048,
    )
    .await?;
    anyhow::ensure!(text.lines().filter(|l| l.trim().split(' ').count() == 2).count() >= 3, "unexpected shape: {text}");
    Ok(())
  }
}

#[cfg(test)]
mod live_new_params {
  use super::*;
  use crate::api::requests::responses::await_response::{await_response, AwaitResponseArgs, PollConfig};
  use crate::api::types::response_types::response_status::ResponseStatus;

  /// Live: reasoning.effort + text.format json_schema round-trip — proves the
  /// new wire fields are accepted and honored. Costs well under $0.01.
  #[tokio::test]
  #[ignore] // live — costs money
  async fn live_structured_output_with_reasoning_effort() -> anyhow::Result<()> {
    crate::test_utils::setup_test_logging::setup_test_logging();
    let key = crate::test_utils::get_test_api_key::get_test_api_key()?;
    let mut req = ChatGpt5_4NanoRequest::new(vec![ChatMessage::user(
      "Alice meets Bob on 2026-07-23. Extract the date.",
    )]);
    req.reasoning_effort = Some(ReasoningEffort::Low);
    req.response_json_schema = Some(JsonSchemaFormat {
      name: "date_extraction".to_string(),
      schema: client_core::JsonSchemaDef::new(serde_json::json!({
        "type": "object",
        "properties": { "date": { "type": "string" } },
        "required": ["date"],
        "additionalProperties": false
      }))
      .unwrap(),
      strict: true,
    });
    let started = chat_gpt_5_4_nano(ChatGpt5_4NanoArgs { api_key: &key, request: req })
      .await
      .map_err(|e| anyhow::anyhow!("{e}"))?;
    let done = await_response(
      AwaitResponseArgs { api_key: &key, response_id: &started.response_id },
      PollConfig::default(),
    )
    .await
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    anyhow::ensure!(done.status == ResponseStatus::Completed, "status: {:?} ({:?})", done.status, done.failure);
    let text = done.output_text.unwrap_or_default();
    let parsed: serde_json::Value = serde_json::from_str(&text)?;
    anyhow::ensure!(parsed["date"].as_str().unwrap_or("").contains("2026-07-23"), "got: {text}");
    Ok(())
  }
}
