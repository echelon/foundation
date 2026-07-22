//! Shared core for the per-model Veo bindings: the `predictLongRunning` wire
//! shape, body assembly, and the start call. Veo is natively asynchronous —
//! starting a generation returns an operation name immediately, which the
//! caller polls with [`crate::api::requests::videos::poll_video_operation`]
//! and downloads with [`crate::api::requests::files::download_video`].
//!
//! Generated videos persist on Google's servers for ~2 days — download and
//! re-host promptly.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use tracing::info;

use client_core::http_client;

use crate::api::requests::google_gemini_host::{GEMINI_API_BASE_URL, GEMINI_USER_AGENT};
use crate::api::types::video_types::veo_image_input::VeoImageInput;
use crate::creds::google_gemini_api_key::GoogleGeminiApiKey;
use crate::error::classify_google_gemini_http_error::classify_google_gemini_http_error;
use crate::error::google_gemini_client_error::GoogleGeminiClientError;
use crate::error::google_gemini_error::GoogleGeminiError;
use crate::error::google_gemini_generic_api_error::GoogleGeminiGenericApiError;

// ── Wire: request ──
//
// {
//   "instances": [ { "prompt": "...", "image": {"bytesBase64Encoded": "...", "mimeType": "image/png"} } ],
//   "parameters": { "aspectRatio": "16:9", "resolution": "720p", "durationSeconds": 8,
//                   "negativePrompt": "..." }   (durationSeconds is a NUMBER — docs say string, API disagrees)
// }

#[derive(Serialize, Debug)]
pub(crate) struct PredictLongRunningBody {
  pub instances: Vec<VeoInstance>,
  pub parameters: VeoParametersWire,
}

#[derive(Serialize, Debug)]
pub(crate) struct VeoInstance {
  pub prompt: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub image: Option<VeoImageWire>,
}

#[derive(Serialize, Debug)]
pub(crate) struct VeoImageWire {
  #[serde(rename = "bytesBase64Encoded", skip_serializing_if = "Option::is_none")]
  pub bytes_base64_encoded: Option<String>,
  #[serde(rename = "fileUri", skip_serializing_if = "Option::is_none")]
  pub file_uri: Option<String>,
  #[serde(rename = "mimeType")]
  pub mime_type: String,
}

#[derive(Serialize, Debug)]
pub(crate) struct VeoParametersWire {
  #[serde(rename = "aspectRatio", skip_serializing_if = "Option::is_none")]
  pub aspect_ratio: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub resolution: Option<String>,
  #[serde(rename = "durationSeconds", skip_serializing_if = "Option::is_none")]
  pub duration_seconds: Option<i64>,
  #[serde(rename = "negativePrompt", skip_serializing_if = "Option::is_none")]
  pub negative_prompt: Option<String>,
}

// ── Shared public types ──

/// The handle a start call returns — poll it to completion. Wire form is the
/// operation resource name, e.g. `models/veo-3.1-generate-preview/operations/abc123`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VeoOperationName(pub String);

#[derive(Deserialize, Debug)]
struct StartResponseBody {
  name: String,
}

/// Assemble the exact outbound body — separate from the send so shape tests
/// can assert it byte-for-byte.
pub(crate) fn build_veo_body(
  prompt: &str,
  image: Option<&VeoImageInput>,
  aspect_ratio: Option<&str>,
  resolution: Option<&str>,
  duration_seconds: Option<i64>,
  negative_prompt: Option<&str>,
) -> PredictLongRunningBody {
  let image = image.map(|i| match i {
    VeoImageInput::FileUri { uri, mime } => VeoImageWire {
      bytes_base64_encoded: None,
      file_uri: Some(uri.clone()),
      mime_type: mime.clone(),
    },
    VeoImageInput::InlineBytes { mime, bytes } => VeoImageWire {
      bytes_base64_encoded: Some(STANDARD.encode(bytes)),
      file_uri: None,
      mime_type: mime.clone(),
    },
  });
  PredictLongRunningBody {
    instances: vec![VeoInstance { prompt: prompt.to_string(), image }],
    parameters: VeoParametersWire {
      aspect_ratio: aspect_ratio.map(str::to_string),
      resolution: resolution.map(str::to_string),
      duration_seconds,
      negative_prompt: negative_prompt.map(str::to_string),
    },
  }
}

/// `POST /models/{model}:predictLongRunning` — start a Veo generation and
/// return the operation name to poll.
pub(crate) async fn run_predict_long_running(
  api_key: &GoogleGeminiApiKey,
  model_id: &str,
  body: &PredictLongRunningBody,
) -> Result<VeoOperationName, GoogleGeminiError> {
  let body_json = serde_json::to_string(body).map_err(GoogleGeminiClientError::RequestSerializationError)?;
  let url = format!("{GEMINI_API_BASE_URL}/models/{model_id}:predictLongRunning");

  info!(model = model_id, "Veo predictLongRunning start");
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
  info!(status = %status, model = model_id, "Veo predictLongRunning response");
  classify_google_gemini_http_error(status, Some(&response_body))?;

  let parsed: StartResponseBody = serde_json::from_str(&response_body)
    .map_err(|e| GoogleGeminiGenericApiError::SerdeResponseParseErrorWithBody(e, response_body))?;
  Ok(VeoOperationName(parsed.name))
}
