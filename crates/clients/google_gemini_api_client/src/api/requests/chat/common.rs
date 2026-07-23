//! Shared chat (text LLM) plumbing: each binding builds an interaction body
//! from its messages — BACKGROUND (handle + poll, like the OpenAI chat
//! bindings) where Google allows it, synchronous where the model is gated out
//! of background mode — and shares one token-based cost estimator.

use client_core::Micros;

use crate::api::requests::interactions::common::{
  GenerationConfigWire, InputItem, InteractionRequestBody, InteractionSuccess, InteractionUsage,
};
use crate::api::types::chat_types::gemini_chat_message::GeminiChatMessage;
use crate::api::types::chat_types::gemini_chat_tool::GeminiChatTool;
use crate::api::types::chat_types::thinking_level::ThinkingLevel;
use crate::error::google_gemini_client_error::GoogleGeminiClientError;
use crate::error::google_gemini_error::GoogleGeminiError;
use crate::error::google_gemini_generic_api_error::GoogleGeminiGenericApiError;

/// Default assumed output length when `max_output_tokens` is unset — chat cost
/// can only be ESTIMATED pre-call (the real output length is unknown until the
/// response).
pub(crate) const DEFAULT_ASSUMED_OUTPUT_TOKENS: u64 = 512;

/// The knobs every chat binding shares (each binding pins its own model id).
#[derive(Clone, Debug)]
pub(crate) struct ChatKnobs<'a> {
  pub messages: &'a [GeminiChatMessage],
  pub system_instruction: Option<&'a str>,
  pub temperature: Option<f32>,
  pub top_p: Option<f32>,
  pub max_output_tokens: Option<u32>,
  pub thinking_level: Option<ThinkingLevel>,
  pub tools: &'a [GeminiChatTool],
  pub response_json_schema: Option<&'a serde_json::Value>,
  pub previous_interaction_id: Option<&'a str>,
  pub store: Option<bool>,
}

pub(crate) fn build_chat_body(
  model_id: &str,
  knobs: &ChatKnobs<'_>,
  background: bool,
) -> Result<InteractionRequestBody, GoogleGeminiClientError> {
  if knobs.messages.is_empty() {
    return Err(GoogleGeminiClientError::InvalidRequest("messages must not be empty".into()));
  }
  // A single user turn rides as a bare text item; longer histories replay as
  // role-tagged message items.
  let input: Vec<InputItem> = if knobs.messages.len() == 1 {
    vec![InputItem::text(&knobs.messages[0].text)]
  } else {
    knobs
      .messages
      .iter()
      .map(|m| InputItem::message(m.role.as_str(), vec![InputItem::text(&m.text)]))
      .collect()
  };
  let has_config = knobs.temperature.is_some()
    || knobs.top_p.is_some()
    || knobs.max_output_tokens.is_some()
    || knobs.thinking_level.is_some()
    || knobs.response_json_schema.is_some();
  Ok(InteractionRequestBody {
    model: model_id.to_string(),
    input,
    system_instruction: knobs.system_instruction.map(str::to_string),
    tools: if knobs.tools.is_empty() { None } else { Some(knobs.tools.to_vec()) },
    response_format: None,
    generation_config: has_config.then(|| GenerationConfigWire {
      video_config: None,
      temperature: knobs.temperature,
      top_p: knobs.top_p,
      max_output_tokens: knobs.max_output_tokens,
      thinking_level: knobs.thinking_level,
      response_json_schema: knobs.response_json_schema.cloned(),
    }),
    previous_interaction_id: knobs.previous_interaction_id.map(str::to_string),
    store: knobs.store,
    background: background.then_some(true),
  })
}

/// A completed chat interaction, extracted.
#[derive(Clone, Debug)]
pub struct ChatSuccess {
  /// The model's text (all model_output text items joined).
  pub text: String,
  /// Pass back as `previous_interaction_id` to continue the conversation
  /// server-side.
  pub interaction_id: Option<String>,
  /// Provider-reported token usage (thinking bills as output) — the
  /// authoritative numbers behind the pre-call estimate.
  pub usage: Option<InteractionUsage>,
}

/// Map a COMPLETED interaction's items into chat output (used after a
/// successful poll).
pub fn chat_success_from_interaction(interaction: InteractionSuccess) -> Result<ChatSuccess, GoogleGeminiError> {
  match interaction.joined_text() {
    Some(text) if !text.is_empty() => {
      Ok(ChatSuccess { text, interaction_id: interaction.interaction_id, usage: interaction.usage })
    }
    _ => Err(
      GoogleGeminiGenericApiError::EmptyResponse {
        detail: "interaction succeeded but carried no text items".to_string(),
      }
      .into(),
    ),
  }
}

/// Estimated cost = (est. input tokens × input rate) + (assumed output tokens ×
/// output rate). Input estimated at ~4 chars/token; output assumes
/// `max_output_tokens` or the default. ESTIMATE ONLY — thinking tokens bill as
/// output, and the response usage is authoritative.
pub(crate) fn estimate_chat_micros(
  micros_per_mtok_in: i64,
  micros_per_mtok_out: i64,
  messages: &[GeminiChatMessage],
  system_instruction: Option<&str>,
  max_output_tokens: Option<u32>,
) -> Micros {
  let mut input_tokens: u64 = messages.iter().map(|m| m.estimated_tokens()).sum();
  input_tokens += system_instruction.map(|s| (s.len() as u64).div_ceil(4)).unwrap_or(0);
  let output_tokens = max_output_tokens.map(u64::from).unwrap_or(DEFAULT_ASSUMED_OUTPUT_TOKENS);
  let input_micros = (input_tokens as i64) * micros_per_mtok_in / 1_000_000;
  let output_micros = (output_tokens as i64) * micros_per_mtok_out / 1_000_000;
  Micros(input_micros + output_micros)
}
