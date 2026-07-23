//! Shared core for bindings on the Interactions API (`POST /v1beta/interactions`)
//! — the newer Gemini surface that Lyria music and Gemini Omni video ship on.
//! One request/response wire shape; each model binding pins its model id and
//! narrows the parameters.
//!
//! Synchronous by provider design (like image generation): the call holds the
//! connection while the model renders — run it from a worker. Omni's `uri`
//! delivery softens this: large outputs come back as a Files-API handle to
//! poll + download instead of inline bytes.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use tracing::info;

use client_core::http_client;

use crate::api::requests::google_gemini_host::{GEMINI_API_BASE_URL, GEMINI_USER_AGENT};
use crate::api::types::image_types::image_input::ImageInput;
use crate::creds::google_gemini_api_key::GoogleGeminiApiKey;
use crate::error::classify_google_gemini_http_error::classify_google_gemini_http_error;
use crate::error::google_gemini_client_error::GoogleGeminiClientError;
use crate::error::google_gemini_error::GoogleGeminiError;
use crate::error::google_gemini_generic_api_error::GoogleGeminiGenericApiError;

// ── Wire: request ──
//
// {
//   "model": "lyria-3-clip-preview",
//   "input": [ {"type": "text", "text": "..."},
//              {"type": "image", "mime_type": "image/png", "data": "<b64>"},
//              {"type": "document", "uri": "files/abc"} ],
//   "response_format": { "type": "video", "aspect_ratio": "16:9", "delivery": "uri" },
//   "generation_config": { "video_config": { "task": "text_to_video" } },
//   "previous_interaction_id": "…", "store": true
// }

#[derive(Serialize, Debug)]
pub(crate) struct InteractionRequestBody {
  pub model: String,
  pub input: Vec<InputItem>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub system_instruction: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub tools: Option<Vec<crate::api::types::chat_types::gemini_chat_tool::GeminiChatTool>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub response_format: Option<ResponseFormatWire>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub generation_config: Option<GenerationConfigWire>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub previous_interaction_id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub store: Option<bool>,
  /// Async mode: the POST returns immediately with an id + `in_progress`; poll
  /// with [`crate::api::requests::interactions::get_interaction`]. Incompatible
  /// with `store: false`.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub background: Option<bool>,
}

#[derive(Serialize, Debug)]
pub(crate) struct InputItem {
  #[serde(rename = "type")]
  pub kind: &'static str,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub text: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub mime_type: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub data: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub uri: Option<String>,
  /// `message` items carry a role + nested content (chat history replay).
  #[serde(skip_serializing_if = "Option::is_none")]
  pub role: Option<&'static str>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub content: Option<Vec<InputItem>>,
}

impl InputItem {
  pub fn text(text: &str) -> Self {
    Self { kind: "text", text: Some(text.to_string()), ..Self::empty() }
  }

  /// A role-tagged chat turn wrapping nested content items.
  pub fn message(role: &'static str, content: Vec<InputItem>) -> Self {
    Self { kind: "message", role: Some(role), content: Some(content), ..Self::empty() }
  }

  fn empty() -> Self {
    Self { kind: "text", text: None, mime_type: None, data: None, uri: None, role: None, content: None }
  }

  /// An image input part. Files-API refs ride as `document` items with a
  /// `uri`; raw bytes become inline base64 `image` items.
  pub fn image(input: &ImageInput) -> Self {
    match input {
      ImageInput::FileUri { uri, mime } => Self {
        kind: "document",
        mime_type: Some(mime.clone()),
        uri: Some(uri.clone()),
        ..Self::empty()
      },
      ImageInput::InlineBytes { mime, bytes } => Self {
        kind: "image",
        mime_type: Some(mime.clone()),
        data: Some(STANDARD.encode(bytes)),
        ..Self::empty()
      },
    }
  }

  /// A video reference (a Files-API upload) — used by Omni edit tasks.
  pub fn video_file(uri: &str) -> Self {
    Self { kind: "document", uri: Some(uri.to_string()), ..Self::empty() }
  }
}

#[derive(Serialize, Debug)]
pub(crate) struct ResponseFormatWire {
  #[serde(rename = "type")]
  pub kind: &'static str,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub aspect_ratio: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub delivery: Option<String>,
}

#[derive(Serialize, Debug, Default)]
pub(crate) struct GenerationConfigWire {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub video_config: Option<VideoConfigWire>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub temperature: Option<f32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub top_p: Option<f32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub max_output_tokens: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub thinking_level: Option<crate::api::types::chat_types::thinking_level::ThinkingLevel>,
  /// Structured output: a JSON Schema the response must conform to.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub response_json_schema: Option<client_core::JsonSchemaDef>,
}

#[derive(Serialize, Debug)]
pub(crate) struct VideoConfigWire {
  pub task: &'static str,
}

// ── Wire: response (tolerant — the surface is in preview) ──

#[derive(Deserialize, Debug)]
pub(crate) struct InteractionResponseBody {
  pub(crate) id: Option<String>,
  pub(crate) status: Option<String>,
  #[serde(default)]
  steps: Vec<StepIn>,
  /// Some responses carry outputs at the top level instead of inside steps.
  #[serde(default, alias = "output", alias = "outputs")]
  content: Vec<ContentItemIn>,
  pub(crate) error: Option<InteractionErrorIn>,
  usage: Option<InteractionUsageIn>,
}

#[derive(Deserialize, Debug)]
struct InteractionUsageIn {
  total_tokens: Option<i64>,
  total_input_tokens: Option<i64>,
  total_output_tokens: Option<i64>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct InteractionErrorIn {
  pub(crate) message: Option<String>,
}

#[derive(Deserialize, Debug)]
struct StepIn {
  /// `user_input`, `thought`, `model_output`, … — only model_output carries
  /// generated artifacts (completed background interactions echo the inputs).
  #[serde(rename = "type")]
  kind: Option<String>,
  #[serde(default)]
  content: Vec<ContentItemIn>,
}

#[derive(Deserialize, Debug)]
struct ContentItemIn {
  #[serde(rename = "type")]
  kind: Option<String>,
  text: Option<String>,
  #[serde(alias = "mimeType")]
  mime_type: Option<String>,
  data: Option<String>,
  uri: Option<String>,
}

// ── Shared public response types ──

/// One output item from an interaction, normalized: inline payloads decoded,
/// URI payloads passed through for the Files download flow.
#[derive(Clone, Debug)]
pub struct InteractionItem {
  /// The wire `type` — `"audio"`, `"video"`, `"text"`, `"image"`, …
  pub kind: String,
  pub mime: Option<String>,
  /// Decoded bytes when the payload came inline.
  pub bytes: Option<Vec<u8>>,
  /// Files-API handle (e.g. `files/abc`) when delivery was by URI — poll with
  /// [`crate::api::requests::files::get_file`] and fetch with
  /// [`crate::api::requests::files::download_file`].
  pub uri: Option<String>,
  pub text: Option<String>,
}

/// Token usage as the Interactions API reported it — the AUTHORITATIVE
/// numbers for reconciling estimated (token-billed) costs post-call.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InteractionUsage {
  pub total_tokens: Option<i64>,
  pub total_input_tokens: Option<i64>,
  pub total_output_tokens: Option<i64>,
}

/// A completed interaction.
#[derive(Clone, Debug)]
pub struct InteractionSuccess {
  /// Pass back as `previous_interaction_id` for stateful/conversational edits.
  pub interaction_id: Option<String>,
  /// Every model_output content item, in order.
  pub items: Vec<InteractionItem>,
  /// Provider-reported token usage, when present.
  pub usage: Option<InteractionUsage>,
}

impl InteractionSuccess {
  /// All items of one kind (e.g. `"audio"`, `"video"`).
  pub fn items_of_kind(&self, kind: &str) -> Vec<&InteractionItem> {
    self.items.iter().filter(|i| i.kind == kind).collect()
  }

  /// Concatenated text items (Lyria uses these for lyrics).
  pub fn joined_text(&self) -> Option<String> {
    let parts: Vec<&str> = self.items.iter().filter_map(|i| i.text.as_deref()).collect();
    if parts.is_empty() { None } else { Some(parts.join("\n")) }
  }
}

/// Normalize a parsed interaction body into items. Only `model_output` steps
/// (and legacy top-level content) count — completed background interactions
/// echo the caller's `user_input` step, which must never leak into results.
pub(crate) fn items_from_body(parsed: InteractionResponseBody) -> Result<InteractionSuccess, GoogleGeminiError> {
  let usage = parsed.usage.as_ref().map(|u| InteractionUsage {
    total_tokens: u.total_tokens,
    total_input_tokens: u.total_input_tokens,
    total_output_tokens: u.total_output_tokens,
  });
  let mut items = Vec::new();
  let step_content = parsed
    .steps
    .into_iter()
    .filter(|s| s.kind.as_deref().is_none_or(|k| k == "model_output"))
    .flat_map(|s| s.content);
  let all = parsed.content.into_iter().chain(step_content);
  for item in all {
    let bytes = match &item.data {
      Some(b64) => Some(
        STANDARD
          .decode(b64.as_bytes())
          .map_err(|e| GoogleGeminiGenericApiError::Base64DecodeError { detail: format!("interaction data: {e}") })?,
      ),
      None => None,
    };
    items.push(InteractionItem {
      kind: item.kind.unwrap_or_else(|| "unknown".to_string()),
      mime: item.mime_type,
      bytes,
      uri: item.uri,
      text: item.text,
    });
  }
  Ok(InteractionSuccess { interaction_id: parsed.id, items, usage })
}

/// The handle a background start returns — poll it with
/// [`crate::api::requests::interactions::get_interaction`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InteractionHandle(pub String);

/// `POST /v1beta/interactions` with `background: true` — start an async
/// interaction and return its id immediately.
pub(crate) async fn run_background_interaction(
  api_key: &GoogleGeminiApiKey,
  body: &InteractionRequestBody,
) -> Result<InteractionHandle, GoogleGeminiError> {
  if body.store == Some(false) {
    return Err(
      GoogleGeminiClientError::InvalidRequest("background interactions require store != false".to_string()).into(),
    );
  }
  let parsed = post_interaction(api_key, body).await?;
  match parsed.id {
    Some(id) => Ok(InteractionHandle(id)),
    None => Err(
      GoogleGeminiGenericApiError::EmptyResponse { detail: "background start returned no interaction id".to_string() }
        .into(),
    ),
  }
}

/// Shared POST + parse for both the sync and background paths.
pub(crate) async fn post_interaction(
  api_key: &GoogleGeminiApiKey,
  body: &InteractionRequestBody,
) -> Result<InteractionResponseBody, GoogleGeminiError> {
  let body_json = serde_json::to_string(body).map_err(GoogleGeminiClientError::RequestSerializationError)?;
  let url = format!("{GEMINI_API_BASE_URL}/interactions");

  info!(model = %body.model, "Gemini interaction start");
  let client = http_client(GEMINI_USER_AGENT);
  let response = client
    .post(&url)
    .header("x-goog-api-key", &api_key.api_key)
    .header("Content-Type", "application/json")
    .body(body_json)
    .send()
    .await
    .map_err(GoogleGeminiGenericApiError::ReqwestError)?;

  let status = response.status();
  let response_body = response.text().await.map_err(GoogleGeminiGenericApiError::ReqwestError)?;
  info!(status = %status, model = %body.model, "Gemini interaction response");
  classify_google_gemini_http_error(status, Some(&response_body))?;

  serde_json::from_str(&response_body)
    .map_err(|e| GoogleGeminiGenericApiError::SerdeResponseParseErrorWithBody(e, response_body).into())
}

/// `POST /v1beta/interactions` — send one SYNCHRONOUS interaction and
/// normalize the output.
#[allow(dead_code)] // kept for models without background support joining this surface later
pub(crate) async fn run_interaction(
  api_key: &GoogleGeminiApiKey,
  body: &InteractionRequestBody,
) -> Result<InteractionSuccess, GoogleGeminiError> {
  let parsed = post_interaction(api_key, body).await?;
  let success = items_from_body(parsed)?;
  if success.items.is_empty() {
    return Err(
      GoogleGeminiGenericApiError::EmptyResponse { detail: "interaction returned no content items".to_string() }
        .into(),
    );
  }
  Ok(success)
}
