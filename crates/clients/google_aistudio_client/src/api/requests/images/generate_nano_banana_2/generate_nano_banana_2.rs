//! `POST /models/gemini-3.1-flash-image:generateContent` — **Nano Banana 2** (`gemini-3.1-flash-image`).
//! The generalist workhorse: balances speed with state-of-the-art 4K output,
//! world knowledge, reliable text rendering, and strong multi-reference
//! consistency.
//! One binding covers BOTH modalities: pure text-to-image (leave `images`
//! empty) and image editing / reference generation (attach [`ImageInput`]s).
//! Synchronous by provider design — run it from a worker.
//!
//! Docs: <https://ai.google.dev/gemini-api/docs/image-generation>

use serde::Serialize;

use crate::api::requests::images::common::{build_image_body, run_generate_content};
use crate::api::types::image_types::image_aspect_ratio::ImageAspectRatio;
use crate::api::types::image_types::image_input::ImageInput;
use crate::api::types::image_types::nano_banana_2_image_size::NanoBanana2ImageSize;
use crate::creds::google_aistudio_api_key::GoogleAiStudioApiKey;
use crate::error::google_aistudio_error::GoogleAiStudioError;

pub use crate::api::requests::images::common::{GeneratedImage, GenerateImageSuccess};

/// The exact model identifier this binding pins.
pub const NANO_BANANA_2_MODEL_ID: &str = "gemini-3.1-flash-image";

/// Top-level argument to [`generate_nano_banana_2`]. Borrows the API key separately
/// from the request body so callers can log/persist (and PRICE) the request
/// without touching the credential.
#[derive(Clone, Debug)]
pub struct GenerateNanoBanana2Args<'a> {
  pub api_key: &'a GoogleAiStudioApiKey,
  pub request: NanoBanana2Request,
}

/// The material part of a Nano Banana 2 request — drives both the wire body and the
/// [`client_core::CostCalculator`] price (see `cost.rs`).
#[derive(Clone, Debug, Serialize)]
pub struct NanoBanana2Request {
  /// Text prompt. Required.
  pub prompt: String,

  /// Edit / reference inputs — empty for pure text-to-image. Prefer
  /// [`ImageInput::FileUri`] (Files API) over inline bytes.
  #[serde(skip_serializing_if = "Vec::is_empty", default)]
  pub images: Vec<ImageInput>,

  /// Aspect ratio; server default (1:1) when `None`.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub aspect_ratio: Option<ImageAspectRatio>,

  /// Output size tier — drives the price. Server default (1K) when `None`.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub image_size: Option<NanoBanana2ImageSize>,
}

/// Generate (or edit) an image with Nano Banana 2. Returns the decoded image bytes.
pub async fn generate_nano_banana_2(args: GenerateNanoBanana2Args<'_>) -> Result<GenerateImageSuccess, GoogleAiStudioError> {
  let GenerateNanoBanana2Args { api_key, request } = args;
  let body = build_image_body(&request.prompt, &request.images, request.aspect_ratio, request.image_size.map(|s| s.as_str()));
  run_generate_content(api_key, NANO_BANANA_2_MODEL_ID, &body).await
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;
  use crate::api::requests::images::common::build_image_body;

  /// Input → exact outbound HTTP body.
  #[test]
  fn text_to_image_wire_shape() {
    let request = NanoBanana2Request {
      prompt: "a lighthouse at dusk".to_string(),
      images: vec![],
      aspect_ratio: Some(ImageAspectRatio::Landscape16x9),
      image_size: Some(NanoBanana2ImageSize::K2),
    };
    let body = build_image_body(&request.prompt, &request.images, request.aspect_ratio, request.image_size.map(|s| s.as_str()));
    assert_eq!(
      serde_json::to_value(&body).unwrap(),
      json!({
        "contents": [{ "parts": [{ "text": "a lighthouse at dusk" }] }],
        "generationConfig": {
          "responseModalities": ["IMAGE"],
          "imageConfig": { "aspectRatio": "16:9", "imageSize": "2K" }
        }
      })
    );
  }

  /// Edit inputs ride as `file_data` parts (Files API URIs) — never data: URLs.
  #[test]
  fn edit_wire_shape_uses_file_data() {
    let request = NanoBanana2Request {
      prompt: "make it snow".to_string(),
      images: vec![ImageInput::FileUri {
        uri: "https://generativelanguage.googleapis.com/v1beta/files/abc123".to_string(),
        mime: "image/png".to_string(),
      }],
      aspect_ratio: None,
      image_size: None,
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

  /// Live call — costs real money (~$0.07 (1K)). Reads
  /// `external/credentials/google_aistudio_api_key.txt`.
  #[tokio::test]
  #[ignore] // live — costs money
  async fn live_generate_nano_banana_2() -> anyhow::Result<()> {
    crate::test_utils::setup_test_logging::setup_test_logging();
    let api_key = crate::test_utils::get_test_api_key::get_test_api_key()?;
    let request = NanoBanana2Request {
      prompt: "A tiny watercolor sailboat on a calm sea, morning light".to_string(),
      images: vec![],
      aspect_ratio: Some(ImageAspectRatio::Square),
      image_size: None,
    };
    let success = generate_nano_banana_2(GenerateNanoBanana2Args { api_key: &api_key, request }).await?;
    assert!(!success.images.is_empty(), "expected at least one image");
    assert!(success.images[0].mime.starts_with("image/"));
    assert!(success.images[0].bytes.len() > 1_000, "suspiciously small image");
    Ok(())
  }
}
