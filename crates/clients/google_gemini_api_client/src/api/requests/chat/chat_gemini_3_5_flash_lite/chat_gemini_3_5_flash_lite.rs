//! `POST /v1beta/interactions` (background) — **Gemini 3.5 Flash-Lite** (`gemini-3.5-flash-lite`),
//! type `chat`. 1M-token context, 65K output. Supports system instructions,
//! sampling knobs, thinking levels, tools (functions + Google-hosted search /
//! code execution / URL context), structured output via JSON Schema, and
//! server-side multi-turn via `previous_interaction_id`.
//!
//! SYNCHRONOUS by provider gating: Google serves this model on the
//! Interactions API but rejects `background: true` for it (as of 2026-07,
//! with a misleading `not_found` error), so the call holds the connection for
//! the (short) render — run it from a worker. Flash-Lite responses are fast;
//! this is the model's operational envelope.
//!
//! Docs: <https://ai.google.dev/gemini-api/docs/models/gemini-3.5-flash-lite>

use serde::Serialize;

use crate::api::requests::chat::common::{build_chat_body, ChatKnobs};
use crate::api::requests::interactions::common::run_interaction;
use crate::api::types::chat_types::gemini_chat_message::GeminiChatMessage;
use crate::api::types::chat_types::gemini_chat_tool::GeminiChatTool;
use crate::api::types::chat_types::thinking_level::ThinkingLevel;
use crate::creds::google_gemini_api_key::GoogleGeminiApiKey;
use crate::error::google_gemini_error::GoogleGeminiError;

pub use crate::api::requests::chat::common::{chat_success_from_interaction, ChatSuccess};

/// The exact model identifier this binding pins.
pub const GEMINI_3_5_FLASH_LITE_MODEL_ID: &str = "gemini-3.5-flash-lite";

/// Top-level argument to [`chat_gemini_3_5_flash_lite`]. Borrows the API key separately from
/// the request body so callers can log/persist (and PRICE) the request without
/// touching the credential.
#[derive(Clone, Debug)]
pub struct ChatGemini35FlashLiteArgs<'a> {
  pub api_key: &'a GoogleGeminiApiKey,
  pub request: Gemini35FlashLiteRequest,
}

/// The material part of a Gemini 3.5 Flash-Lite request — drives both the wire body and the
/// [`client_core::CostCalculator`] estimate (see `cost.rs`).
#[derive(Clone, Debug, Serialize)]
pub struct Gemini35FlashLiteRequest {
  /// The conversation so far (at least one turn). A single user turn is sent
  /// as plain text; longer histories replay as role-tagged messages.
  pub messages: Vec<GeminiChatMessage>,

  /// Standing behavior/persona instructions.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub system_instruction: Option<String>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub temperature: Option<f32>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub top_p: Option<f32>,

  /// Cap on generated tokens (also drives the pre-call cost estimate).
  #[serde(skip_serializing_if = "Option::is_none")]
  pub max_output_tokens: Option<u32>,

  /// Reasoning effort. Text models document `low`/`high`.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub thinking_level: Option<ThinkingLevel>,

  /// Tools the model may call (functions + Google-hosted built-ins).
  #[serde(skip_serializing_if = "Vec::is_empty", default)]
  pub tools: Vec<GeminiChatTool>,

  /// Structured output: a JSON Schema the response must conform to.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub response_json_schema: Option<serde_json::Value>,

  /// Continue a stored server-side conversation.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub previous_interaction_id: Option<String>,
}

/// Run a chat completion (synchronous — see module docs). Returns the reply.
pub async fn chat_gemini_3_5_flash_lite(args: ChatGemini35FlashLiteArgs<'_>) -> Result<ChatSuccess, GoogleGeminiError> {
  let ChatGemini35FlashLiteArgs { api_key, request } = args;
  let body = build_chat_body(
    GEMINI_3_5_FLASH_LITE_MODEL_ID,
    &ChatKnobs {
      messages: &request.messages,
      system_instruction: request.system_instruction.as_deref(),
      temperature: request.temperature,
      top_p: request.top_p,
      max_output_tokens: request.max_output_tokens,
      thinking_level: request.thinking_level,
      tools: &request.tools,
      response_json_schema: request.response_json_schema.as_ref(),
      previous_interaction_id: request.previous_interaction_id.as_deref(),
    },
    false,
  )?;
  let interaction = run_interaction(api_key, &body).await?;
  chat_success_from_interaction(interaction)
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;
  use crate::api::types::chat_types::gemini_chat_message::GeminiChatRole;

  fn knobs_of(request: &Gemini35FlashLiteRequest) -> ChatKnobs<'_> {
    ChatKnobs {
      messages: &request.messages,
      system_instruction: request.system_instruction.as_deref(),
      temperature: request.temperature,
      top_p: request.top_p,
      max_output_tokens: request.max_output_tokens,
      thinking_level: request.thinking_level,
      tools: &request.tools,
      response_json_schema: request.response_json_schema.as_ref(),
      previous_interaction_id: request.previous_interaction_id.as_deref(),
    }
  }

  /// Single-turn input → exact outbound HTTP body.
  #[test]
  fn single_turn_wire_shape() {
    let request = Gemini35FlashLiteRequest {
      messages: vec![GeminiChatMessage::user("What is the tallest mountain?")],
      system_instruction: Some("Answer in one short sentence.".to_string()),
      temperature: Some(0.5),
      top_p: None,
      max_output_tokens: Some(256),
      thinking_level: Some(ThinkingLevel::Low),
      tools: vec![],
      response_json_schema: None,
      previous_interaction_id: None,
    };
    let body = build_chat_body(GEMINI_3_5_FLASH_LITE_MODEL_ID, &knobs_of(&request), false).unwrap();
    assert_eq!(
      serde_json::to_value(&body).unwrap(),
      json!({
        "model": "gemini-3.5-flash-lite",
        "input": [{ "type": "text", "text": "What is the tallest mountain?" }],
        "system_instruction": "Answer in one short sentence.",
        "generation_config": { "temperature": 0.5, "max_output_tokens": 256, "thinking_level": "low" }
      })
    );
  }

  /// Multi-turn history replays as role-tagged messages; tools + schema ride along.
  #[test]
  fn multi_turn_wire_shape_with_tools() {
    let request = Gemini35FlashLiteRequest {
      messages: vec![
        GeminiChatMessage::user("hi"),
        GeminiChatMessage { role: GeminiChatRole::Model, text: "hello!".to_string() },
        GeminiChatMessage::user("search for rust news"),
      ],
      system_instruction: None,
      temperature: None,
      top_p: None,
      max_output_tokens: None,
      thinking_level: None,
      tools: vec![GeminiChatTool::GoogleSearch],
      response_json_schema: None,
      previous_interaction_id: None,
    };
    let body = build_chat_body(GEMINI_3_5_FLASH_LITE_MODEL_ID, &knobs_of(&request), false).unwrap();
    assert_eq!(
      serde_json::to_value(&body).unwrap(),
      json!({
        "model": "gemini-3.5-flash-lite",
        "input": [
          { "type": "message", "role": "user", "content": [{ "type": "text", "text": "hi" }] },
          { "type": "message", "role": "model", "content": [{ "type": "text", "text": "hello!" }] },
          { "type": "message", "role": "user", "content": [{ "type": "text", "text": "search for rust news" }] }
        ],
        "tools": [{ "type": "google_search" }]
      })
    );
  }

  /// Live start→poll — costs fractions of a cent. Reads
  /// `external/credentials/google_gemini_api_key.txt`.
  #[tokio::test]
  #[ignore] // live — costs money (well under $0.01)
  async fn live_chat_gemini_3_5_flash_lite() -> anyhow::Result<()> {
    crate::test_utils::setup_test_logging::setup_test_logging();
    let api_key = crate::test_utils::get_test_api_key::get_test_api_key()?;
    let request = Gemini35FlashLiteRequest {
      messages: vec![GeminiChatMessage::user("Reply with exactly the word: pineapple")],
      system_instruction: None,
      temperature: Some(0.0),
      top_p: None,
      max_output_tokens: Some(2000),
      thinking_level: None,
      tools: vec![],
      response_json_schema: None,
      previous_interaction_id: None,
    };
    let chat = chat_gemini_3_5_flash_lite(ChatGemini35FlashLiteArgs { api_key: &api_key, request }).await?;
    assert!(chat.text.to_lowercase().contains("pineapple"), "unexpected reply: {}", chat.text);
    Ok(())
  }
}
