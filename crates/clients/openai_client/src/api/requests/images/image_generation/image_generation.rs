use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Serialize;
use tracing::info;

use crate::api::requests::images::image_generation::request_types::*;
use crate::api::requests::openai_host::{with_auth, OPENAI_API_BASE_URL};
use crate::api::types::image_types::image_model::ImageModel;
use crate::api::types::image_types::image_quality::ImageQuality;
use crate::api::types::image_types::image_size::ImageSize;
use crate::api::types::image_types::output_format::OutputFormat;
use crate::creds::openai_api_key::OpenAiApiKey;
use crate::error::classify_openai_http_error::classify_openai_http_error;
use crate::error::openai_client_error::OpenAiClientError;
use crate::error::openai_error::OpenAiError;
use crate::error::openai_generic_api_error::OpenAiGenericApiError;

// ── Public args ──

/// Argument to [`image_generation`]. Borrows the key separately from the request body so
/// callers can log/persist [`ImageGenerationRequest`] without leaking the credential.
#[derive(Clone, Debug)]
pub struct ImageGenerationArgs<'a> {
  pub api_key: &'a OpenAiApiKey,
  pub request: ImageGenerationRequest,
}

/// The material part of an image-generation request. Serializes cleanly (no key).
#[derive(Clone, Debug, Serialize)]
pub struct ImageGenerationRequest {
  pub prompt: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub model: Option<ImageModel>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub number_images: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub size: Option<ImageSize>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub quality: Option<ImageQuality>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub output_format: Option<OutputFormat>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub transparent_background: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub user: Option<String>,
}

impl ImageGenerationRequest {
  pub fn new(prompt: impl Into<String>) -> Self {
    Self {
      prompt: prompt.into(),
      model: None,
      number_images: None,
      size: None,
      quality: None,
      output_format: None,
      transparent_background: None,
      user: None,
    }
  }
}

// ── Public response ──

#[derive(Debug, Clone)]
pub struct ImageGenerationSuccess {
  pub images: Vec<GeneratedImage>,
  pub usage: ImagesUsage,
}

#[derive(Debug, Clone)]
pub struct GeneratedImage {
  /// MIME of the image, e.g. `image/png`.
  pub mime: String,
  /// Decoded image bytes — GPT Image always returns base64, so this is normally `Some`.
  pub bytes: Option<Vec<u8>>,
  /// A URL to the image (DALL·E-compat responses); GPT Image doesn't use this.
  pub url: Option<String>,
  pub revised_prompt: Option<String>,
}

// ── Implementation ──

/// `POST https://api.openai.com/v1/images/generations` — text → image(s).
///
/// Docs: <https://developers.openai.com/api/reference/resources/images/methods/generate>
pub async fn image_generation(args: ImageGenerationArgs<'_>) -> Result<ImageGenerationSuccess, OpenAiError> {
  let req = args.request;

  // ── client-side validation ──
  if req.prompt.trim().is_empty() {
    return Err(OpenAiClientError::InvalidRequest("prompt must not be empty".into()).into());
  }
  if let Some(n) = req.number_images {
    if !(1..=10).contains(&n) {
      return Err(OpenAiClientError::InvalidRequest(format!("n must be 1..=10, got {n}")).into());
    }
  }

  let model = req.model.clone().unwrap_or(ImageModel::GptImage1);
  let out_fmt = req.output_format.unwrap_or(OutputFormat::Png);

  info!(
    model = model.as_str(),
    n = req.number_images.unwrap_or(1),
    "openai image_generation"
  );

  let body = ImageGenerationRequestBody {
    model: model.as_str().to_string(),
    prompt: req.prompt,
    n: req.number_images,
    size: req.size.map(|s| s.as_str().to_string()),
    quality: req.quality.map(|q| q.as_str().to_string()),
    background: req.transparent_background.map(|t| if t { "transparent".into() } else { "opaque".into() }),
    output_format: Some(out_fmt.as_str().to_string()),
    moderation: None,
    user: req.user,
  };

  let client = client_core::http_client("foundation/openai-client");
  let builder = client
    .post(format!("{OPENAI_API_BASE_URL}/v1/images/generations"))
    .header("Content-Type", "application/json")
    .json(&body);

  let response = with_auth(builder, args.api_key)
    .send()
    .await
    .map_err(OpenAiGenericApiError::ReqwestError)?;

  let status = response.status();
  let text = response.text().await.map_err(OpenAiGenericApiError::ReqwestError)?;
  info!(%status, "openai image_generation response");

  classify_openai_http_error(status, Some(&text))?;

  let parsed: ImagesResponseBody = serde_json::from_str(&text)
    .map_err(|e| OpenAiGenericApiError::SerdeResponseParseErrorWithBody(e, text.clone()))?;

  let mime = OutputFormat::from_wire(parsed.output_format.as_deref().unwrap_or("png")).mime();
  let images = decode_images(parsed.data, mime)?;

  Ok(ImageGenerationSuccess {
    images,
    usage: parsed.usage.unwrap_or_default(),
  })
}

/// Decode the response `data[]` into normalized [`GeneratedImage`]s (shared with edit).
pub(crate) fn decode_images(
  data: Vec<ImagesResponseDatum>,
  mime: &str,
) -> Result<Vec<GeneratedImage>, OpenAiError> {
  let mut out = Vec::with_capacity(data.len());
  for d in data {
    let (bytes, url) = if let Some(b64) = d.b64_json {
      let decoded = STANDARD
        .decode(b64.as_bytes())
        .map_err(|e| OpenAiGenericApiError::SerdeResponseParseErrorWithBody(serde_error(&e), b64.clone()))?;
      (Some(decoded), None)
    } else if let Some(url) = d.url {
      (None, Some(url))
    } else {
      return Err(OpenAiGenericApiError::UncategorizedBadResponseWithStatusAndBody {
        status_code: reqwest::StatusCode::OK,
        body: "image datum had neither b64_json nor url".into(),
      }
      .into());
    };
    out.push(GeneratedImage { mime: mime.to_string(), bytes, url, revised_prompt: d.revised_prompt });
  }
  Ok(out)
}

// base64 decode errors aren't serde errors; wrap them so they still carry the raw payload.
fn serde_error(e: &base64::DecodeError) -> serde_json::Error {
  serde::de::Error::custom(format!("base64 decode failed: {e}"))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn request_serializes_typed_enums_no_key() {
    let req = ImageGenerationRequest {
      prompt: "a serene lake".into(),
      model: Some(ImageModel::GptImage2),
      number_images: Some(2),
      size: Some(ImageSize::Landscape1536x1024),
      quality: Some(ImageQuality::High),
      output_format: Some(OutputFormat::Png),
      transparent_background: None,
      user: None,
    };
    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("\"model\":\"gpt-image-2\""));
    assert!(json.contains("\"quality\":\"high\""));
    assert!(json.contains("\"size\":\"1536x1024\""));
    assert!(!json.contains("api_key"));
  }

  #[test]
  fn wire_body_skips_none_fields() {
    let body = ImageGenerationRequestBody {
      model: "gpt-image-1".into(),
      prompt: "cat".into(),
      n: None,
      size: None,
      quality: None,
      background: None,
      output_format: Some("png".into()),
      moderation: None,
      user: None,
    };
    let json = serde_json::to_string(&body).unwrap();
    assert!(json.contains("\"model\":\"gpt-image-1\""));
    assert!(json.contains("\"output_format\":\"png\""));
    assert!(!json.contains("\"n\""));
    assert!(!json.contains("\"quality\""));
  }

  #[test]
  fn decode_images_b64_to_bytes() {
    let data = vec![ImagesResponseDatum {
      b64_json: Some(STANDARD.encode(b"hello")),
      url: None,
      revised_prompt: Some("p".into()),
    }];
    let imgs = decode_images(data, "image/png").unwrap();
    assert_eq!(imgs.len(), 1);
    assert_eq!(imgs[0].mime, "image/png");
    assert_eq!(imgs[0].bytes.as_deref(), Some(b"hello".as_slice()));
    assert!(imgs[0].url.is_none());
  }

  #[tokio::test]
  #[ignore] // live — requires OPENAI_API_KEY, incurs cost
  async fn live_image_generation() -> anyhow::Result<()> {
    use crate::test_utils::get_test_api_key::get_test_api_key;
    use crate::test_utils::setup_test_logging::setup_test_logging;
    setup_test_logging();
    let key = get_test_api_key()?;
    let res = image_generation(ImageGenerationArgs {
      api_key: &key,
      request: ImageGenerationRequest {
        prompt: "A serene mountain lake at sunrise, photorealistic".into(),
        model: Some(ImageModel::GptImage1),
        number_images: Some(1),
        size: Some(ImageSize::Square1024),
        quality: Some(ImageQuality::Low),
        output_format: Some(OutputFormat::Png),
        transparent_background: None,
        user: None,
      },
    })
    .await
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    assert_eq!(res.images.len(), 1);
    Ok(())
  }
}
