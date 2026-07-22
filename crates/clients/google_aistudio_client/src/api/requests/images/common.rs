//! Shared core for the per-model image bindings: the `generateContent` wire
//! shape, body assembly, send, and response decoding. Each model binding pins
//! its own model id, request struct, and price — this module is the one place
//! that knows the HTTP/JSON mechanics.
//!
//! Synchronous by provider design: the Gemini API has no async/poll variant for
//! interactive image generation (only the 24-hour Batch API), so the call holds
//! the connection for the duration of the render — callers should run it from a
//! worker, never from a request handler that's holding a client connection.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use tracing::info;

use client_core::http_client;

use crate::api::requests::aistudio_host::{GEMINI_API_BASE_URL, GEMINI_USER_AGENT};
use crate::api::types::image_types::image_aspect_ratio::ImageAspectRatio;
use crate::api::types::image_types::image_input::ImageInput;
use crate::creds::google_aistudio_api_key::GoogleAiStudioApiKey;
use crate::error::classify_google_aistudio_http_error::classify_google_aistudio_http_error;
use crate::error::google_aistudio_client_error::GoogleAiStudioClientError;
use crate::error::google_aistudio_error::GoogleAiStudioError;
use crate::error::google_aistudio_generic_api_error::GoogleAiStudioGenericApiError;

// ── Wire: request ──
//
// {
//   "contents": [ { "parts": [ {"text": "..."},
//                              {"inline_data": {"mime_type": "image/png", "data": "<b64>"}},
//                              {"file_data": {"mime_type": "image/png", "file_uri": "https://…/files/x"}} ] } ],
//   "generationConfig": {
//     "responseModalities": ["IMAGE"],
//     "imageConfig": { "aspectRatio": "16:9", "imageSize": "2K" }
//   }
// }

#[derive(Serialize, Debug)]
pub(crate) struct GenerateContentRequestBody {
  pub contents: Vec<Content>,
  #[serde(rename = "generationConfig")]
  pub generation_config: GenerationConfig,
}

#[derive(Serialize, Debug)]
pub(crate) struct Content {
  pub parts: Vec<PartOut>,
}

#[derive(Serialize, Debug, Default)]
pub(crate) struct PartOut {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub text: Option<String>,
  #[serde(rename = "inline_data", skip_serializing_if = "Option::is_none")]
  pub inline_data: Option<InlineDataOut>,
  #[serde(rename = "file_data", skip_serializing_if = "Option::is_none")]
  pub file_data: Option<FileDataOut>,
}

#[derive(Serialize, Debug)]
pub(crate) struct InlineDataOut {
  pub mime_type: String,
  pub data: String,
}

#[derive(Serialize, Debug)]
pub(crate) struct FileDataOut {
  pub mime_type: String,
  pub file_uri: String,
}

#[derive(Serialize, Debug)]
pub(crate) struct GenerationConfig {
  #[serde(rename = "responseModalities")]
  pub response_modalities: Vec<String>,
  #[serde(rename = "imageConfig", skip_serializing_if = "Option::is_none")]
  pub image_config: Option<ImageConfig>,
}

#[derive(Serialize, Debug)]
pub(crate) struct ImageConfig {
  #[serde(rename = "aspectRatio", skip_serializing_if = "Option::is_none")]
  pub aspect_ratio: Option<String>,
  #[serde(rename = "imageSize", skip_serializing_if = "Option::is_none")]
  pub image_size: Option<String>,
}

// ── Wire: response ──

#[derive(Deserialize, Debug)]
pub(crate) struct GenerateContentResponseBody {
  pub candidates: Option<Vec<Candidate>>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct Candidate {
  pub content: Option<ContentIn>,
  #[serde(rename = "finishReason")]
  pub finish_reason: Option<String>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct ContentIn {
  #[serde(default)]
  pub parts: Vec<PartIn>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct PartIn {
  #[serde(rename = "inlineData")]
  pub inline_data: Option<InlineDataIn>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct InlineDataIn {
  #[serde(rename = "mimeType")]
  pub mime_type: String,
  pub data: String,
}

// ── Shared public response types ──

/// A single generated image, decoded from the inline base64 the API returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedImage {
  /// MIME type of the bytes, e.g. `image/png`.
  pub mime: String,
  /// Decoded image bytes.
  pub bytes: Vec<u8>,
}

/// The outcome of one image generation call.
#[derive(Debug, Clone)]
pub struct GenerateImageSuccess {
  /// One entry per generated image, in the order Google returned them.
  pub images: Vec<GeneratedImage>,
  /// The `finishReason` reported per candidate (e.g. `"STOP"`), in order.
  pub finish_reasons: Vec<Option<String>>,
}

/// Build the exact outbound body for a generate call — separate from the send
/// so shape tests can assert it byte-for-byte.
pub(crate) fn build_image_body(
  prompt: &str,
  images: &[ImageInput],
  aspect_ratio: Option<ImageAspectRatio>,
  image_size: Option<&str>,
) -> GenerateContentRequestBody {
  let mut parts: Vec<PartOut> = vec![PartOut { text: Some(prompt.to_string()), ..Default::default() }];
  for img in images {
    parts.push(match img {
      ImageInput::FileUri { uri, mime } => PartOut {
        file_data: Some(FileDataOut { mime_type: mime.clone(), file_uri: uri.clone() }),
        ..Default::default()
      },
      ImageInput::InlineBytes { mime, bytes } => PartOut {
        inline_data: Some(InlineDataOut { mime_type: mime.clone(), data: STANDARD.encode(bytes) }),
        ..Default::default()
      },
    });
  }
  let image_config = if aspect_ratio.is_some() || image_size.is_some() {
    Some(ImageConfig {
      aspect_ratio: aspect_ratio.map(|a| a.as_str().to_string()),
      image_size: image_size.map(str::to_string),
    })
  } else {
    None
  };
  GenerateContentRequestBody {
    contents: vec![Content { parts }],
    generation_config: GenerationConfig { response_modalities: vec!["IMAGE".to_string()], image_config },
  }
}

/// `POST /models/{model}:generateContent` — the shared send + decode path every
/// per-model binding funnels through.
pub(crate) async fn run_generate_content(
  api_key: &GoogleAiStudioApiKey,
  model_id: &str,
  body: &GenerateContentRequestBody,
) -> Result<GenerateImageSuccess, GoogleAiStudioError> {
  let body_json = serde_json::to_string(body).map_err(GoogleAiStudioClientError::RequestSerializationError)?;
  let url = format!("{GEMINI_API_BASE_URL}/models/{model_id}:generateContent");

  info!(model = model_id, "Gemini generateContent (image)");
  let client = http_client(GEMINI_USER_AGENT);
  let response = client
    .post(&url)
    .header("x-goog-api-key", &api_key.api_key)
    .header("Content-Type", "application/json")
    .body(body_json)
    .send()
    .await
    .map_err(GoogleAiStudioGenericApiError::ReqwestError)?;

  let status = response.status();
  let response_body = response.text().await.map_err(GoogleAiStudioGenericApiError::ReqwestError)?;
  info!(status = %status, model = model_id, "Gemini generateContent response");
  classify_google_aistudio_http_error(status, Some(&response_body))?;

  let parsed: GenerateContentResponseBody = serde_json::from_str(&response_body)
    .map_err(|e| GoogleAiStudioGenericApiError::SerdeResponseParseErrorWithBody(e, response_body))?;

  let mut images = Vec::new();
  let mut finish_reasons = Vec::new();
  for candidate in parsed.candidates.unwrap_or_default() {
    finish_reasons.push(candidate.finish_reason);
    for part in candidate.content.map(|c| c.parts).unwrap_or_default() {
      if let Some(inline) = part.inline_data {
        let bytes = STANDARD
          .decode(inline.data.as_bytes())
          .map_err(|e| GoogleAiStudioGenericApiError::Base64DecodeError { detail: format!("inlineData: {e}") })?;
        images.push(GeneratedImage { mime: inline.mime_type, bytes });
      }
    }
  }
  if images.is_empty() {
    return Err(
      GoogleAiStudioGenericApiError::EmptyResponse {
        detail: format!("no image parts in response (finish_reasons: {finish_reasons:?} — safety block?)"),
      }
      .into(),
    );
  }
  Ok(GenerateImageSuccess { images, finish_reasons })
}
