//! Wire shapes for `POST /v1/responses` (create, background) and `GET /v1/responses/{id}`
//! (poll) — shared by every model binding. Bindings build a [`CreateResponseBody`] and tests
//! assert its exact JSON, so the input→outbound-HTTP mapping is pinned per binding.

use serde::{Deserialize, Serialize};

use crate::api::types::chat_types::chat_role::ChatRole;
use crate::api::types::chat_types::reasoning_effort::ReasoningEffort;
use crate::api::types::image_types::image_action::ImageAction;
use crate::api::types::image_types::image_background::ImageBackground;
use crate::api::types::image_types::image_moderation::ImageModeration;
use crate::api::types::image_types::image_quality::ImageQuality;
use crate::api::types::image_types::image_size::ImageSize;
use crate::api::types::image_types::input_fidelity::InputFidelity;
use crate::api::types::image_types::output_format::OutputFormat;

// ── Create ──

#[derive(Debug, Serialize)]
pub(crate) struct CreateResponseBody {
  pub model: String,
  pub input: Vec<InputMessage>,
  /// Always true — we never hold a request open; create returns immediately and we poll.
  pub background: bool,
  /// Background mode requires the response to be stored (OpenAI keeps it ~briefly for
  /// polling; incompatible with Zero Data Retention orgs).
  pub store: bool,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub max_output_tokens: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub temperature: Option<f32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub top_p: Option<f32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub user: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub tools: Option<Vec<ImageGenerationTool>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub tool_choice: Option<ForcedToolChoice>,
  /// `reasoning: {"effort": …}` — GPT-5-family reasoning depth.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub reasoning: Option<ReasoningWire>,
  /// `text: {"format": {json_schema…}}` — structured output.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub text: Option<TextWire>,
  /// Continue a stored conversation server-side.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub previous_response_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ReasoningWire {
  pub effort: ReasoningEffort,
}

#[derive(Debug, Serialize)]
pub(crate) struct TextWire {
  pub format: JsonSchemaFormatWire,
}

#[derive(Debug, Serialize)]
pub(crate) struct JsonSchemaFormatWire {
  #[serde(rename = "type")]
  pub kind: &'static str, // always "json_schema"
  pub name: String,
  pub schema: serde_json::Value,
  pub strict: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct InputMessage {
  pub role: ChatRole,
  pub content: MessageContent,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub(crate) enum MessageContent {
  /// Plain text (LLM chat).
  Text(String),
  /// Structured parts (image prompt + input images).
  Parts(Vec<ContentPart>),
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub(crate) enum ContentPart {
  #[serde(rename = "input_text")]
  Text { text: String },
  /// Exactly one of `image_url` (https or data URL) / `file_id` is set.
  #[serde(rename = "input_image")]
  Image {
    #[serde(skip_serializing_if = "Option::is_none")]
    image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    file_id: Option<String>,
  },
}

/// The `image_generation` tool config — where the image model and its knobs live.
#[derive(Debug, Serialize)]
pub(crate) struct ImageGenerationTool {
  #[serde(rename = "type")]
  pub kind: &'static str, // always "image_generation"
  pub model: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub size: Option<ImageSize>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub quality: Option<ImageQuality>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub output_format: Option<OutputFormat>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub input_fidelity: Option<InputFidelity>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub input_image_mask: Option<MaskReference>,
  /// `auto` (default) / `generate` / `edit`.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub action: Option<ImageAction>,
  /// Output transparency (gpt-image-2 rejects `transparent`).
  #[serde(skip_serializing_if = "Option::is_none")]
  pub background: Option<ImageBackground>,
  /// Content-filter strictness.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub moderation: Option<ImageModeration>,
  /// 0–100, jpeg/webp outputs only.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub output_compression: Option<u8>,
}

#[derive(Debug, Serialize)]
pub(crate) struct MaskReference {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub image_url: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub file_id: Option<String>,
}

/// `tool_choice: {"type": "image_generation"}` — force the image tool (no driver small talk).
#[derive(Debug, Serialize)]
pub(crate) struct ForcedToolChoice {
  #[serde(rename = "type")]
  pub kind: &'static str,
}

// ── Retrieve / poll ──

#[derive(Debug, Deserialize)]
pub(crate) struct ResponseWire {
  pub id: String,
  pub status: String,
  #[serde(default)]
  pub model: Option<String>,
  #[serde(default)]
  pub output: Vec<OutputItemWire>,
  #[serde(default)]
  pub error: Option<ResponseErrorWire>,
  #[serde(default)]
  pub incomplete_details: Option<IncompleteDetailsWire>,
  #[serde(default)]
  pub usage: Option<ResponseUsageWire>,
}

/// One output item. Kept as a loose struct (not a tagged enum) so unknown item types are
/// skipped instead of failing the whole parse.
#[derive(Debug, Deserialize)]
pub(crate) struct OutputItemWire {
  #[serde(rename = "type")]
  pub kind: String,
  #[serde(default)]
  pub content: Vec<OutputContentWire>,
  /// `image_generation_call`: the base64 image.
  #[serde(default)]
  pub result: Option<String>,
  #[serde(default)]
  pub output_format: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OutputContentWire {
  #[serde(rename = "type")]
  pub kind: String,
  #[serde(default)]
  pub text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ResponseErrorWire {
  #[serde(default)]
  pub code: Option<String>,
  #[serde(default)]
  pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct IncompleteDetailsWire {
  #[serde(default)]
  pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ResponseUsageWire {
  #[serde(default)]
  pub input_tokens: i64,
  #[serde(default)]
  pub output_tokens: i64,
  #[serde(default)]
  pub total_tokens: i64,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn completed_chat_response_parses() {
    let json = r#"{
      "id": "resp_1", "status": "completed", "model": "gpt-5.5",
      "output": [
        { "type": "reasoning", "summary": [] },
        { "type": "message", "role": "assistant",
          "content": [ { "type": "output_text", "text": "Hello!" } ] }
      ],
      "usage": { "input_tokens": 10, "output_tokens": 3, "total_tokens": 13 }
    }"#;
    let parsed: ResponseWire = serde_json::from_str(json).unwrap();
    assert_eq!(parsed.status, "completed");
    assert_eq!(parsed.output.len(), 2);
    assert_eq!(parsed.output[1].content[0].text.as_deref(), Some("Hello!"));
    assert_eq!(parsed.usage.unwrap().total_tokens, 13);
  }

  #[test]
  fn completed_image_response_parses() {
    let json = r#"{
      "id": "resp_2", "status": "completed",
      "output": [
        { "type": "image_generation_call", "id": "ig_1", "status": "completed",
          "result": "aGVsbG8=", "output_format": "png" }
      ]
    }"#;
    let parsed: ResponseWire = serde_json::from_str(json).unwrap();
    assert_eq!(parsed.output[0].kind, "image_generation_call");
    assert_eq!(parsed.output[0].result.as_deref(), Some("aGVsbG8="));
  }

  #[test]
  fn queued_and_failed_responses_parse() {
    let queued: ResponseWire = serde_json::from_str(r#"{ "id": "resp_3", "status": "queued" }"#).unwrap();
    assert!(queued.output.is_empty());
    let failed: ResponseWire = serde_json::from_str(
      r#"{ "id": "resp_4", "status": "failed", "error": { "code": "server_error", "message": "boom" } }"#,
    )
    .unwrap();
    assert_eq!(failed.error.unwrap().code.as_deref(), Some("server_error"));
  }
}
