//! `POST /v1beta/interactions` — **Gemini Omni Flash** (`gemini-omni-flash-preview`),
//! the multimodal omni video model: high-speed video generation, editing, and
//! conversational cinematic control. Text/image/video inputs in one request;
//! chain edits with `previous_interaction_id`.
//!
//! ASYNC: starts a background interaction and returns the handle immediately —
//! poll with [`crate::api::requests::interactions::get_interaction`], then map
//! the completed items with [`omni_video_success_from_interaction`]. No
//! connection is held while the model renders. (`store: false` is incompatible
//! with background mode and is rejected client-side.)
//!
//! Delivery: default here is [`OmniDelivery::Uri`] — the completed interaction
//! carries a direct download link for
//! [`crate::api::requests::files::download_file`] instead of inline base64
//! (which only works ≤ 4 MB).
//!
//! Docs: <https://ai.google.dev/gemini-api/docs/omni>

use serde::Serialize;

use crate::api::requests::interactions::common::{
  run_background_interaction, GenerationConfigWire, InputItem, InteractionRequestBody, InteractionSuccess,
  InteractionUsage, ResponseFormatWire, VideoConfigWire,
};
use crate::api::types::image_types::image_input::ImageInput;
use crate::api::types::video_types::omni_aspect_ratio::OmniAspectRatio;
use crate::api::types::video_types::omni_delivery::OmniDelivery;
use crate::api::types::video_types::omni_video_task::OmniVideoTask;
use crate::creds::google_gemini_api_key::GoogleGeminiApiKey;
use crate::error::google_gemini_error::GoogleGeminiError;
use crate::error::google_gemini_generic_api_error::GoogleGeminiGenericApiError;

/// The exact model identifier this binding pins.
pub const GEMINI_OMNI_FLASH_MODEL_ID: &str = "gemini-omni-flash-preview";

pub use crate::api::requests::interactions::common::InteractionHandle;

/// Top-level argument to [`generate_gemini_omni_flash`]. Borrows the API key
/// separately from the request body so callers can log/persist (and PRICE) the
/// request without touching the credential.
#[derive(Clone, Debug)]
pub struct GenerateGeminiOmniFlashArgs<'a> {
  pub api_key: &'a GoogleGeminiApiKey,
  pub request: GeminiOmniFlashRequest,
}

/// The material part of an Omni request — drives both the wire body and the
/// [`client_core::CostCalculator`] estimate (see `cost.rs`).
#[derive(Clone, Debug, Serialize)]
pub struct GeminiOmniFlashRequest {
  /// The direction — subject, camera work, edits. Negative guidance goes in
  /// the prompt too (no discrete negative-prompt parameter).
  pub prompt: String,

  /// Conditioning / reference images. Prefer [`ImageInput::FileUri`].
  #[serde(skip_serializing_if = "Vec::is_empty", default)]
  pub images: Vec<ImageInput>,

  /// A Files-API video reference (`files/…`) to edit — for
  /// [`OmniVideoTask::Edit`] on uploaded footage.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub video_file_uri: Option<String>,

  /// What kind of video work this is; server infers when `None`.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub task: Option<OmniVideoTask>,

  /// 16:9 (default) or 9:16.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub aspect_ratio: Option<OmniAspectRatio>,

  /// How the video comes back. `None` = [`OmniDelivery::Uri`] (this binding's
  /// default — inline only fits ≤ 4 MB).
  #[serde(skip_serializing_if = "Option::is_none")]
  pub delivery: Option<OmniDelivery>,

  /// Chain onto a prior interaction for conversational editing.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub previous_interaction_id: Option<String>,

  /// `false` opts out of server-side storage (disables follow-up edits).
  #[serde(skip_serializing_if = "Option::is_none")]
  pub store: Option<bool>,

  /// ESTIMATION ONLY (never sent): expected output length in seconds for the
  /// cost calculator — Omni is token-billed (~$0.10/s of 720p video) and the
  /// length is prompt-driven, so the price is an estimate by nature. `None`
  /// estimates 8 s.
  #[serde(skip_serializing)]
  pub expected_output_seconds: Option<i64>,
}

/// One finished Omni video.
#[derive(Clone, Debug)]
pub enum OmniVideoOutput {
  /// Inline delivery (≤ 4 MB), decoded.
  Inline { mime: String, bytes: Vec<u8> },
  /// URI delivery: a DIRECT download link — pass straight to `download_file`
  /// (empirically the file is ready when the interaction returns; no polling).
  FileUri(String),
}

/// The outcome of one Omni generation.
#[derive(Clone, Debug)]
pub struct GenerateOmniVideoSuccess {
  pub videos: Vec<OmniVideoOutput>,
  /// Any accompanying text the model produced.
  pub text: Option<String>,
  /// Pass back as `previous_interaction_id` for conversational edits.
  pub interaction_id: Option<String>,
  /// Provider-reported token usage — the authoritative basis of Omni's
  /// token-billed cost (the pre-call number is only an estimate).
  pub usage: Option<InteractionUsage>,
}

/// Start a background video generation/edit. Returns the interaction handle
/// to poll with `get_interaction`.
pub async fn generate_gemini_omni_flash(
  args: GenerateGeminiOmniFlashArgs<'_>,
) -> Result<InteractionHandle, GoogleGeminiError> {
  let GenerateGeminiOmniFlashArgs { api_key, request } = args;
  let body = build_body(&request);
  run_background_interaction(api_key, &body).await
}

/// The EXACT provider-native JSON body this request would send — same
/// construction path as the real send (router logging / dry-run contract).
/// NOTE: for upload-first inputs the `images` present here are whatever the
/// request carries at call time; the dispatcher logs the post-upload body too.
pub fn generate_gemini_omni_flash_wire_json(
  request: &GeminiOmniFlashRequest,
) -> Result<serde_json::Value, GoogleGeminiError> {
  let body = build_body(request);
  serde_json::to_value(&body).map_err(|e| {
    crate::error::google_gemini_client_error::GoogleGeminiClientError::RequestSerializationError(e).into()
  })
}

/// Map a COMPLETED interaction's items into video output (used after a
/// successful poll).
pub fn omni_video_success_from_interaction(
  interaction: InteractionSuccess,
) -> Result<GenerateOmniVideoSuccess, GoogleGeminiError> {
  let text = interaction.joined_text();
  let videos: Vec<OmniVideoOutput> = interaction
    .items
    .iter()
    .filter(|i| i.kind == "video" || (i.kind == "document" && i.uri.is_some()))
    .filter_map(|i| match (&i.bytes, &i.uri) {
      (Some(bytes), _) => Some(OmniVideoOutput::Inline {
        mime: i.mime.clone().unwrap_or_else(|| "video/mp4".to_string()),
        bytes: bytes.clone(),
      }),
      (None, Some(uri)) => Some(OmniVideoOutput::FileUri(uri.clone())),
      (None, None) => None,
    })
    .collect();
  if videos.is_empty() {
    return Err(
      GoogleGeminiGenericApiError::EmptyResponse {
        detail: "interaction succeeded but carried no video items".to_string(),
      }
      .into(),
    );
  }
  Ok(GenerateOmniVideoSuccess { videos, text, interaction_id: interaction.interaction_id, usage: interaction.usage })
}

pub(crate) fn build_body(request: &GeminiOmniFlashRequest) -> InteractionRequestBody {
  let mut input = vec![InputItem::text(&request.prompt)];
  input.extend(request.images.iter().map(InputItem::image));
  if let Some(uri) = &request.video_file_uri {
    input.push(InputItem::video_file(uri));
  }
  InteractionRequestBody {
    system_instruction: None,
    tools: None,
    model: GEMINI_OMNI_FLASH_MODEL_ID.to_string(),
    input,
    response_format: Some(ResponseFormatWire {
      kind: "video",
      aspect_ratio: request.aspect_ratio.map(|a| a.as_str().to_string()),
      delivery: Some(request.delivery.unwrap_or(OmniDelivery::Uri).as_str().to_string()),
    }),
    generation_config: request
      .task
      .map(|t| GenerationConfigWire { video_config: Some(VideoConfigWire { task: t.as_str() }), ..Default::default() }),
    previous_interaction_id: request.previous_interaction_id.clone(),
    store: request.store,
    background: Some(true),
  }
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  /// Input → exact outbound HTTP body (URI delivery is the default).
  #[test]
  fn text_to_video_wire_shape() {
    let request = GeminiOmniFlashRequest {
      prompt: "a red kite over dunes, sweeping drone shot".to_string(),
      images: vec![],
      video_file_uri: None,
      task: Some(OmniVideoTask::TextToVideo),
      aspect_ratio: Some(OmniAspectRatio::Landscape16x9),
      delivery: None,
      previous_interaction_id: None,
      store: None,
      expected_output_seconds: None,
    };
    assert_eq!(
      serde_json::to_value(build_body(&request)).unwrap(),
      json!({
        "model": "gemini-omni-flash-preview",
        "input": [{ "type": "text", "text": "a red kite over dunes, sweeping drone shot" }],
        "response_format": { "type": "video", "aspect_ratio": "16:9", "delivery": "uri" },
        "generation_config": { "video_config": { "task": "text_to_video" } },
        "background": true
      })
    );
  }

  /// Reference images ride as Files-API `document` items — never data: URLs.
  #[test]
  fn reference_wire_shape_uses_file_uris() {
    let request = GeminiOmniFlashRequest {
      prompt: "animate in this style".to_string(),
      images: vec![ImageInput::FileUri { uri: "files/ref1".to_string(), mime: "image/png".to_string() }],
      video_file_uri: None,
      task: Some(OmniVideoTask::ReferenceToVideo),
      aspect_ratio: None,
      delivery: Some(OmniDelivery::Uri),
      previous_interaction_id: None,
      store: None,
      expected_output_seconds: None,
    };
    assert_eq!(
      serde_json::to_value(build_body(&request)).unwrap(),
      json!({
        "model": "gemini-omni-flash-preview",
        "input": [
          { "type": "text", "text": "animate in this style" },
          { "type": "document", "mime_type": "image/png", "uri": "files/ref1" }
        ],
        "response_format": { "type": "video", "delivery": "uri" },
        "generation_config": { "video_config": { "task": "reference_to_video" } },
        "background": true
      })
    );
  }

  /// Live generate → (poll file if URI) → download — costs real money
  /// (token-billed, ~$0.10/s of 720p; a short clip ≈ $0.50–1.00). Reads
  /// `external/credentials/gemini_api_key.txt`.
  #[tokio::test]
  #[ignore] // live — costs money
  async fn live_generate_gemini_omni_flash() -> anyhow::Result<()> {
    use crate::api::requests::files::download_file::download_file::{download_file, DownloadFileArgs};

    crate::test_utils::setup_test_logging::setup_test_logging();
    let api_key = crate::test_utils::get_test_api_key::get_test_api_key()?;
    let request = GeminiOmniFlashRequest {
      prompt: "A very short clip, about four seconds: a paper windmill spinning on a windowsill".to_string(),
      images: vec![],
      video_file_uri: None,
      task: Some(OmniVideoTask::TextToVideo),
      aspect_ratio: None,
      delivery: Some(OmniDelivery::Uri),
      previous_interaction_id: None,
      store: None,
      expected_output_seconds: Some(4),
    };
    let handle = generate_gemini_omni_flash(GenerateGeminiOmniFlashArgs { api_key: &api_key, request }).await?;

    // Background: poll (5s cadence, ≤6 min) until terminal, then download.
    use crate::api::requests::interactions::get_interaction::get_interaction::{
      get_interaction, GetInteractionArgs, InteractionPollOutcome,
    };
    for _ in 0..72 {
      tokio::time::sleep(std::time::Duration::from_secs(5)).await;
      match get_interaction(GetInteractionArgs { api_key: &api_key, interaction: &handle }).await? {
        InteractionPollOutcome::InProgress => continue,
        InteractionPollOutcome::Cancelled => anyhow::bail!("interaction cancelled"),
        InteractionPollOutcome::Failed { message } => anyhow::bail!("interaction failed: {message}"),
        InteractionPollOutcome::Completed(success) => {
          let video = omni_video_success_from_interaction(success)?;
          match &video.videos[0] {
            OmniVideoOutput::Inline { bytes, .. } => assert!(bytes.len() > 10_000),
            OmniVideoOutput::FileUri(uri) => {
              let bytes = download_file(DownloadFileArgs { api_key: &api_key, name: uri }).await?;
              assert!(bytes.len() > 10_000, "suspiciously small video");
            }
          }
          return Ok(());
        }
      }
    }
    anyhow::bail!("interaction did not complete within 6 minutes")
  }
}
