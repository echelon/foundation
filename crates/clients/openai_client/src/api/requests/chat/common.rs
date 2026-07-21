//! Shared chat plumbing: every LLM binding builds a background `/v1/responses` body from
//! its messages, and shares one token-based cost estimator.

use client_core::Micros;

use crate::api::requests::responses::create_response::{create_background_response, StartedResponse};
use crate::api::requests::responses::wire::{CreateResponseBody, InputMessage, MessageContent};
use crate::api::types::chat_types::chat_message::ChatMessage;
use crate::api::types::chat_types::chat_model::ChatModel;
use crate::creds::openai_api_key::OpenAiApiKey;
use crate::error::openai_client_error::OpenAiClientError;
use crate::error::openai_error::OpenAiError;

/// Default assumed output length when `max_output_tokens` is unset — chat cost can only be
/// ESTIMATED pre-call (the real output length is unknown until the response).
pub(crate) const DEFAULT_ASSUMED_OUTPUT_TOKENS: u64 = 512;

/// The knobs every chat binding shares (each binding pins its own model).
#[derive(Clone, Debug)]
pub(crate) struct ChatKnobs<'a> {
  pub messages: &'a [ChatMessage],
  pub max_output_tokens: Option<u32>,
  pub temperature: Option<f32>,
  pub top_p: Option<f32>,
  pub user: Option<&'a str>,
}

pub(crate) fn build_chat_body(model: &ChatModel, knobs: &ChatKnobs<'_>) -> Result<CreateResponseBody, OpenAiClientError> {
  if knobs.messages.is_empty() {
    return Err(OpenAiClientError::InvalidRequest("messages must not be empty".into()));
  }
  Ok(CreateResponseBody {
    model: model.as_str().to_string(),
    input: knobs
      .messages
      .iter()
      .map(|m| InputMessage { role: m.role, content: MessageContent::Text(m.content.clone()) })
      .collect(),
    background: true,
    store: true,
    max_output_tokens: knobs.max_output_tokens,
    temperature: knobs.temperature,
    top_p: knobs.top_p,
    user: knobs.user.map(str::to_string),
    tools: None,
    tool_choice: None,
  })
}

pub(crate) async fn start_chat(
  api_key: &OpenAiApiKey,
  model: &ChatModel,
  knobs: &ChatKnobs<'_>,
) -> Result<StartedResponse, OpenAiError> {
  let body = build_chat_body(model, knobs)?;
  create_background_response(api_key, &body).await
}

/// Estimated cost = (est. input tokens × input rate) + (assumed output tokens × output
/// rate). Input tokens estimated at ~4 chars/token; output assumes `max_output_tokens` or
/// the default. ESTIMATE ONLY — the response `usage` is authoritative (and gpt-5.6-sol's
/// >272K-context breakpoint pricing is not modeled).
pub(crate) fn estimate_chat_micros(
  model: &ChatModel,
  messages: &[ChatMessage],
  max_output_tokens: Option<u32>,
) -> Micros {
  let (in_rate, out_rate) = model.pricing_micros_per_mtok();
  let input_tokens: u64 = messages.iter().map(|m| m.estimated_tokens()).sum();
  let output_tokens = max_output_tokens.map(u64::from).unwrap_or(DEFAULT_ASSUMED_OUTPUT_TOKENS);
  let input_micros = (input_tokens as i64) * in_rate / 1_000_000;
  let output_micros = (output_tokens as i64) * out_rate / 1_000_000;
  Micros(input_micros + output_micros)
}
