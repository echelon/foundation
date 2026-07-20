use base64::{Engine as _, engine::general_purpose::STANDARD};
use client_core::Asset;
use serde::Serialize;
use tracing::info;

use crate::api::requests::google_host::GEMINI_API_BASE_URL;
use crate::api::requests::images::generate_image::request_types::*;
use crate::api::types::image_types::gemini_image_model::GeminiImageModel;
use crate::api::types::image_types::image_aspect_ratio::ImageAspectRatio;
use crate::api::types::image_types::image_size::ImageSize;
use crate::creds::gemini_api_key::GeminiApiKey;
use crate::error::classify_google_http_error::classify_google_http_error;
use crate::error::google_error::GoogleError;
use crate::error::google_generic_api_error::GoogleGenericApiError;

// ── Public args ──

/// Top-level argument to [`generate_image`]. Borrows the API key separately
/// from the request body so callers can log/save [`GenerateImageRequest`]
/// without leaking the credential.
#[derive(Clone, Debug)]
pub struct GenerateImageArgs<'a> {
  pub api_key: &'a GeminiApiKey,
  pub request: GenerateImageRequest,
}

/// A single inline image input for edit / reference generation. Becomes an
/// `inline_data` part on the request. Empty for pure text-to-image.
#[derive(Clone, Debug, Serialize)]
pub struct ImageInput {
  /// MIME type of the input bytes, e.g. `"image/png"`.
  pub mime: String,
  /// Raw (un-encoded) image bytes. Base64-encoded on the wire.
  pub data: Vec<u8>,
}

/// The material part of a generate-image request. Derives [`Serialize`] so it
/// can be persisted to a log or audit store independently of the API key.
#[derive(Clone, Debug, Serialize)]
pub struct GenerateImageRequest {
  /// Text prompt describing the image. Required.
  pub prompt: String,

  /// Model identifier. Defaults to [`GeminiImageModel::NanoBanana`]
  /// (`gemini-2.5-flash-image`) when `None`. Use [`GeminiImageModel::Custom`]
  /// for identifiers not yet listed in the enum.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub model: Option<GeminiImageModel>,

  /// Aspect ratio. See [`ImageAspectRatio`]. Server default when `None`.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub aspect_ratio: Option<ImageAspectRatio>,

  /// Output size tier (`1K`/`2K`/`4K`). See [`ImageSize`]. Server default when
  /// `None`.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub image_size: Option<ImageSize>,

  /// Number of images to render (Gemini's `candidateCount`). Server default is
  /// 1 when `None`.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub candidate_count: Option<u32>,

  /// Reference/edit inputs (empty for pure text-to-image). Each becomes an
  /// `inline_data` part.
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub images: Vec<ImageInput>,
}

// ── Public response ──

#[derive(Debug, Clone)]
pub struct GenerateImageSuccess {
  /// One entry per generated image, decoded from the inline base64 into
  /// [`Asset::Bytes`]. Order matches the order Google returned.
  pub images: Vec<Asset>,

  /// The `finishReason` reported for each candidate (e.g. `"STOP"`), in
  /// candidate order.
  pub finish_reasons: Vec<Option<String>>,
}

// ── Implementation ──

/// POST https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent
/// — generate one or more images from a text prompt (plus optional reference
/// images) using Google's Gemini image API (the Nano Banana family).
///
/// Synchronous: the image bytes come back base64-encoded inline and are decoded
/// into [`Asset::Bytes`].
///
/// Docs: <https://ai.google.dev/gemini-api/docs/image-generation>
pub async fn generate_image(args: GenerateImageArgs<'_>) -> Result<GenerateImageSuccess, GoogleError> {
  let req = args.request;

  let model = req.model.unwrap_or(GeminiImageModel::NanoBanana);
  let url = format!("{}/models/{}:generateContent", GEMINI_API_BASE_URL, model.as_str());

  info!(
    "Gemini generate_image: model={}, candidate_count={:?}, aspect_ratio={:?}, image_size={:?}, num_input_images={}",
    model.as_str(),
    req.candidate_count,
    req.aspect_ratio.map(|a| a.as_str()),
    req.image_size.map(|s| s.as_str()),
    req.images.len(),
  );

  // Assemble the `parts`: the text prompt first, then any inline reference
  // images (base64-encoded).
  let mut parts: Vec<PartOut> = vec![PartOut::text(req.prompt)];
  for img in &req.images {
    parts.push(PartOut::inline(img.mime.clone(), STANDARD.encode(&img.data)));
  }

  // `imageConfig` is only sent when at least one of its fields is set.
  let image_config = match (req.aspect_ratio, req.image_size) {
    (None, None) => None,
    (aspect_ratio, image_size) => Some(ImageConfig {
      aspect_ratio: aspect_ratio.map(|a| a.as_str().to_string()),
      image_size: image_size.map(|s| s.as_str().to_string()),
    }),
  };

  let request_body = GenerateContentRequestBody {
    contents: vec![Content { parts }],
    generation_config: GenerationConfig {
      response_modalities: vec!["IMAGE".to_string()],
      candidate_count: req.candidate_count,
      image_config,
    },
  };

  let client = client_core::http_client("foundation/google-client");

  let response = client.post(&url)
    .header("x-goog-api-key", &args.api_key.api_key)
    .header("Content-Type", "application/json")
    .json(&request_body)
    .send()
    .await
    .map_err(GoogleGenericApiError::ReqwestError)?;

  let status = response.status();
  let response_body = response.text()
    .await
    .map_err(GoogleGenericApiError::ReqwestError)?;

  info!("Gemini generate_image response: status={}", status);

  classify_google_http_error(status, Some(&response_body))?;

  let parsed: GenerateContentResponseBody = serde_json::from_str(&response_body)
    .map_err(|err| GoogleGenericApiError::SerdeResponseParseErrorWithBody(err, response_body.clone()))?;

  let mut images = Vec::new();
  let mut finish_reasons = Vec::new();

  for candidate in parsed.candidates.unwrap_or_default() {
    finish_reasons.push(candidate.finish_reason);
    for part in candidate.content.parts {
      if let Some(inline) = part.inline_data {
        let data = STANDARD.decode(inline.data.as_bytes())
          .map_err(|e| GoogleGenericApiError::Base64DecodeError { detail: e.to_string() })?;
        images.push(Asset::Bytes { mime: inline.mime_type, data });
      }
    }
  }

  Ok(GenerateImageSuccess { images, finish_reasons })
}

#[cfg(test)]
mod tests {
  use super::*;

  // ── Wire-format shape tests (internal GenerateContentRequestBody) ──

  #[test]
  fn wire_body_serializes_minimal() {
    let body = GenerateContentRequestBody {
      contents: vec![Content { parts: vec![PartOut::text("a cat".to_string())] }],
      generation_config: GenerationConfig {
        response_modalities: vec!["IMAGE".to_string()],
        candidate_count: None,
        image_config: None,
      },
    };
    let json = serde_json::to_string(&body).unwrap();
    assert!(json.contains("\"text\":\"a cat\""));
    assert!(json.contains("\"responseModalities\":[\"IMAGE\"]"));
    assert!(!json.contains("candidateCount"));
    assert!(!json.contains("imageConfig"));
    assert!(!json.contains("inline_data"));
  }

  #[test]
  fn wire_body_serializes_full() {
    let body = GenerateContentRequestBody {
      contents: vec![Content {
        parts: vec![
          PartOut::text("a cat".to_string()),
          PartOut::inline("image/png".to_string(), "aGVsbG8=".to_string()),
        ],
      }],
      generation_config: GenerationConfig {
        response_modalities: vec!["IMAGE".to_string()],
        candidate_count: Some(2),
        image_config: Some(ImageConfig {
          aspect_ratio: Some("16:9".to_string()),
          image_size: Some("2K".to_string()),
        }),
      },
    };
    let json = serde_json::to_string(&body).unwrap();
    assert!(json.contains("\"candidateCount\":2"));
    assert!(json.contains("\"imageConfig\""));
    assert!(json.contains("\"aspectRatio\":\"16:9\""));
    assert!(json.contains("\"imageSize\":\"2K\""));
    assert!(json.contains("\"inline_data\""));
    assert!(json.contains("\"mime_type\":\"image/png\""));
    assert!(json.contains("\"data\":\"aGVsbG8=\""));
  }

  // ── Public-API shape tests: GenerateImageRequest must serialize cleanly ──

  #[test]
  fn request_serializes_with_typed_enums() {
    let req = GenerateImageRequest {
      prompt: "a serene lake".to_string(),
      model: Some(GeminiImageModel::NanoBananaPro),
      aspect_ratio: Some(ImageAspectRatio::Landscape16x9),
      image_size: Some(ImageSize::TwoK),
      candidate_count: Some(3),
      images: vec![],
    };
    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("\"prompt\":\"a serene lake\""));
    assert!(json.contains("\"model\":\"gemini-3-pro-image\""));
    assert!(json.contains("\"aspect_ratio\":\"16:9\""));
    assert!(json.contains("\"image_size\":\"2K\""));
    assert!(json.contains("\"candidate_count\":3"));
    // Empty images vec is skipped.
    assert!(!json.contains("\"images\""));
  }

  #[test]
  fn request_does_not_serialize_api_key() {
    let key = GeminiApiKey::new("secret_value_must_not_leak".to_string());
    let args = GenerateImageArgs {
      api_key: &key,
      request: GenerateImageRequest {
        prompt: "p".to_string(),
        model: None,
        aspect_ratio: None,
        image_size: None,
        candidate_count: None,
        images: vec![],
      },
    };
    let json = serde_json::to_string(&args.request).unwrap();
    assert!(!json.contains("secret_value_must_not_leak"),
      "serialized request must not contain the API key. got: {}", json);
  }

  // ── Response deserialization ──

  #[test]
  fn response_body_deserializes_inline_image() {
    let json = r#"{
      "candidates": [
        {
          "content": { "parts": [ { "inlineData": { "mimeType": "image/png", "data": "aGVsbG8=" } } ] },
          "finishReason": "STOP"
        }
      ],
      "usageMetadata": { "totalTokenCount": 123 }
    }"#;
    let parsed: GenerateContentResponseBody = serde_json::from_str(json).unwrap();
    let candidates = parsed.candidates.unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].finish_reason.as_deref(), Some("STOP"));
    let inline = candidates[0].content.parts[0].inline_data.as_ref().unwrap();
    assert_eq!(inline.mime_type, "image/png");
    assert_eq!(inline.data, "aGVsbG8=");
  }

  #[test]
  fn response_body_decodes_base64_to_asset_bytes() {
    // "aGVsbG8=" is base64 for "hello".
    let decoded = STANDARD.decode("aGVsbG8=").unwrap();
    assert_eq!(decoded, b"hello");
  }

  #[test]
  fn response_body_handles_missing_candidates() {
    let json = r#"{ "usageMetadata": {} }"#;
    let parsed: GenerateContentResponseBody = serde_json::from_str(json).unwrap();
    assert!(parsed.candidates.is_none());
  }

  // ── Live API tests (ignored — incur cost) ──

  #[tokio::test]
  #[ignore] // manually test — requires real API key and incurs costs
  async fn live_test_generate_image_simple() -> anyhow::Result<()> {
    use crate::test_utils::get_test_api_key::get_test_api_key;
    use crate::test_utils::setup_test_logging::setup_test_logging;
    setup_test_logging();

    let api_key = get_test_api_key()?;
    let result = generate_image(GenerateImageArgs {
      api_key: &api_key,
      request: GenerateImageRequest {
        prompt: "A serene mountain lake at sunrise, photorealistic".to_string(),
        model: None,
        aspect_ratio: Some(ImageAspectRatio::Landscape16x9),
        image_size: Some(ImageSize::OneK),
        candidate_count: None,
        images: vec![],
      },
    }).await.map_err(|e| anyhow::anyhow!("{}", e))?;

    println!("Generated {} image(s), finish_reasons={:?}", result.images.len(), result.finish_reasons);
    assert!(!result.images.is_empty());
    match &result.images[0] {
      Asset::Bytes { mime, data } => {
        println!("  [0] mime={} bytes={}", mime, data.len());
        assert!(!data.is_empty());
      }
      other => panic!("expected Asset::Bytes, got {:?}", other),
    }
    Ok(())
  }
}
