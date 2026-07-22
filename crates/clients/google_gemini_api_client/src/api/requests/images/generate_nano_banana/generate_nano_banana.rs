//! `POST /models/gemini-2.5-flash-image:generateContent` — **Nano Banana** (`gemini-2.5-flash-image`).
//! LEGACY: the pioneer of the Nano Banana series — still reliable, but Google
//! recommends transitioning to Nano Banana 2 Lite for better quality, speed,
//! and price.
//! One binding covers BOTH modalities: pure text-to-image (leave `images`
//! empty) and image editing / reference generation (attach [`ImageInput`]s).
//! Synchronous by provider design — run it from a worker.
//!
//! Docs: <https://ai.google.dev/gemini-api/docs/image-generation>

use serde::Serialize;

use crate::api::requests::images::common::{build_image_body, run_generate_content};
use crate::api::types::image_types::image_aspect_ratio::ImageAspectRatio;
use crate::api::types::image_types::image_input::ImageInput;
use crate::creds::google_gemini_api_key::GoogleGeminiApiKey;
use crate::error::google_gemini_error::GoogleGeminiError;

pub use crate::api::requests::images::common::{GeneratedImage, GenerateImageSuccess};

/// The exact model identifier this binding pins.
pub const NANO_BANANA_MODEL_ID: &str = "gemini-2.5-flash-image";

/// Top-level argument to [`generate_nano_banana`]. Borrows the API key separately
/// from the request body so callers can log/persist (and PRICE) the request
/// without touching the credential.
#[derive(Clone, Debug)]
pub struct GenerateNanoBananaArgs<'a> {
  pub api_key: &'a GoogleGeminiApiKey,
  pub request: NanoBananaRequest,
}

/// The material part of a Nano Banana request — drives both the wire body and the
/// [`client_core::CostCalculator`] price (see `cost.rs`).
#[derive(Clone, Debug, Serialize)]
pub struct NanoBananaRequest {
  /// Text prompt. Required.
  pub prompt: String,

  /// Edit / reference inputs — empty for pure text-to-image. Prefer
  /// [`ImageInput::FileUri`] (Files API) over inline bytes.
  #[serde(skip_serializing_if = "Vec::is_empty", default)]
  pub images: Vec<ImageInput>,

  /// Aspect ratio; server default (1:1) when `None`.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub aspect_ratio: Option<ImageAspectRatio>,
}

/// Generate (or edit) an image with Nano Banana. Returns the decoded image bytes.
pub async fn generate_nano_banana(args: GenerateNanoBananaArgs<'_>) -> Result<GenerateImageSuccess, GoogleGeminiError> {
  let GenerateNanoBananaArgs { api_key, request } = args;
  let body = build_image_body(&request.prompt, &request.images, request.aspect_ratio, None);
  run_generate_content(api_key, NANO_BANANA_MODEL_ID, &body).await
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;
  use crate::api::requests::images::common::build_image_body;

  /// Input → exact outbound HTTP body.
  #[test]
  fn text_to_image_wire_shape() {
    let request = NanoBananaRequest {
      prompt: "a lighthouse at dusk".to_string(),
      images: vec![],
      aspect_ratio: Some(ImageAspectRatio::Landscape16x9),
    };
    let body = build_image_body(&request.prompt, &request.images, request.aspect_ratio, None);
    assert_eq!(
      serde_json::to_value(&body).unwrap(),
      json!({
        "contents": [{ "parts": [{ "text": "a lighthouse at dusk" }] }],
        "generationConfig": {
          "responseModalities": ["IMAGE"],
          "imageConfig": { "aspectRatio": "16:9" }
        }
      })
    );
  }

  /// Edit inputs ride as `file_data` parts (Files API URIs) — never data: URLs.
  #[test]
  fn edit_wire_shape_uses_file_data() {
    let request = NanoBananaRequest {
      prompt: "make it snow".to_string(),
      images: vec![ImageInput::FileUri {
        uri: "https://generativelanguage.googleapis.com/v1beta/files/abc123".to_string(),
        mime: "image/png".to_string(),
      }],
      aspect_ratio: None,
    };
    let body = build_image_body(&request.prompt, &request.images, request.aspect_ratio, None);
    assert_eq!(
      serde_json::to_value(&body).unwrap(),
      json!({
        "contents": [{ "parts": [
          { "text": "make it snow" },
          { "file_data": { "mime_type": "image/png",
                           "file_uri": "https://generativelanguage.googleapis.com/v1beta/files/abc123" } }
        ] }],
        "generationConfig": { "responseModalities": ["IMAGE"] }
      })
    );
  }

  /// Live call — costs real money (~$0.04). Reads
  /// `external/credentials/gemini_api_key.txt`.
  #[tokio::test]
  #[ignore] // live — costs money
  async fn live_generate_nano_banana() -> anyhow::Result<()> {
    crate::test_utils::setup_test_logging::setup_test_logging();
    let api_key = crate::test_utils::get_test_api_key::get_test_api_key()?;
    let request = NanoBananaRequest {
      prompt: "A tiny watercolor sailboat on a calm sea, morning light".to_string(),
      images: vec![],
      aspect_ratio: Some(ImageAspectRatio::Square),
    };
    let success = generate_nano_banana(GenerateNanoBananaArgs { api_key: &api_key, request }).await?;
    assert!(!success.images.is_empty(), "expected at least one image");
    assert!(success.images[0].mime.starts_with("image/"));
    assert!(success.images[0].bytes.len() > 1_000, "suspiciously small image");
    Ok(())
  }
}
