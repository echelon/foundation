//! Shared chat plumbing: every LLM binding builds a background `/v1/responses` body from
//! its messages, and shares one token-based cost estimator.

use client_core::{CostBreakdown, CostComponent, CostFormula, CostQuantity, CostUnit, RateRow};

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

/// Shared chat pricing: input tokens estimated (~4 chars/token) + output at
/// `max_output_tokens` (or the default) — ALWAYS an estimate; the response
/// `usage` is authoritative. One rate card per model (in/out per 1M tokens).
pub(crate) fn chat_cost_breakdown(
  model: &ChatModel,
  messages: &[ChatMessage],
  max_output_tokens: Option<u32>,
) -> CostBreakdown {
  let (in_rate, out_rate) = model.pricing_micros_per_mtok();
  let input_tokens: u64 = messages.iter().map(|m| m.estimated_tokens()).sum();
  let output_tokens = max_output_tokens.map(u64::from).unwrap_or(DEFAULT_ASSUMED_OUTPUT_TOKENS);
  CostBreakdown::from_components(
    CostFormula {
      display: "input_tokens × rate + output_tokens × rate",
      rates: vec![
        RateRow {
          dimension: "tokens",
          value: "input".to_string(),
          unit: CostUnit::PerMillionInputTokens,
          price_micros: in_rate,
        },
        RateRow {
          dimension: "tokens",
          value: "output".to_string(),
          unit: CostUnit::PerMillionOutputTokens,
          price_micros: out_rate,
        },
      ],
    },
    vec![
      CostComponent::new(
        "Input tokens (~4 chars/token estimate)",
        CostUnit::PerMillionInputTokens,
        in_rate,
        CostQuantity::Estimated(input_tokens as f64),
      ),
      CostComponent::new(
        if max_output_tokens.is_some() { "Output tokens (at max_output_tokens cap)" } else { "Output tokens (assumed default)" },
        CostUnit::PerMillionOutputTokens,
        out_rate,
        CostQuantity::Estimated(output_tokens as f64),
      )
      .with_note("output length unknown pre-call; response usage is authoritative"),
    ],
    true,
  )
}
