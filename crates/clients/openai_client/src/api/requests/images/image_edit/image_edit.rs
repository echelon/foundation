use reqwest::multipart::{Form, Part};
use tracing::info;

use crate::api::requests::images::image_edit::request_types::InputImage;
use crate::api::requests::images::image_generation::image_generation::{decode_images, ImageGenerationSuccess};
use crate::api::requests::images::image_generation::request_types::ImagesResponseBody;
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

#[derive(Debug)]
pub struct ImageEditArgs<'a> {
  pub api_key: &'a OpenAiApiKey,
  pub request: ImageEditRequest,
}

/// Instruction-driven edit / multi-reference composite. Not `Serialize` (carries image
/// bytes) — the request is sent as `multipart/form-data`. `Debug` redacts the bytes.
#[derive(Debug)]
pub struct ImageEditRequest {
  pub prompt: String,
  /// Input image(s). At least one required; up to 16 for GPT Image.
  pub images: Vec<InputImage>,
  /// Optional PNG mask marking the editable region.
  pub mask: Option<InputImage>,
  pub model: Option<ImageModel>,
  pub number_images: Option<u32>,
  pub size: Option<ImageSize>,
  pub quality: Option<ImageQuality>,
  pub output_format: Option<OutputFormat>,
  pub user: Option<String>,
}

impl ImageEditRequest {
  pub fn new(prompt: impl Into<String>, images: Vec<InputImage>) -> Self {
    Self {
      prompt: prompt.into(),
      images,
      mask: None,
      model: None,
      number_images: None,
      size: None,
      quality: None,
      output_format: None,
      user: None,
    }
  }
}

// ── Implementation ──

/// `POST https://api.openai.com/v1/images/edits` (multipart) — edit / compose input image(s).
///
/// Docs: <https://developers.openai.com/api/reference/resources/images/methods/edit>
pub async fn image_edit(args: ImageEditArgs<'_>) -> Result<ImageGenerationSuccess, OpenAiError> {
  let req = args.request;

  if req.prompt.trim().is_empty() {
    return Err(OpenAiClientError::InvalidRequest("prompt must not be empty".into()).into());
  }
  if req.images.is_empty() {
    return Err(OpenAiClientError::InvalidRequest("image_edit requires at least one input image".into()).into());
  }
  if req.images.len() > 16 {
    return Err(OpenAiClientError::InvalidRequest(format!("at most 16 input images, got {}", req.images.len())).into());
  }

  let model = req.model.clone().unwrap_or(ImageModel::GptImage1);
  let out_fmt = req.output_format.unwrap_or(OutputFormat::Png);

  info!(model = model.as_str(), images = req.images.len(), "openai image_edit");

  // ── build multipart form ──
  let mut form = Form::new()
    .text("model", model.as_str().to_string())
    .text("prompt", req.prompt)
    .text("output_format", out_fmt.as_str().to_string());

  if let Some(n) = req.number_images {
    form = form.text("n", n.to_string());
  }
  if let Some(s) = req.size {
    form = form.text("size", s.as_str().to_string());
  }
  if let Some(q) = req.quality {
    form = form.text("quality", q.as_str().to_string());
  }
  if let Some(u) = req.user {
    form = form.text("user", u);
  }
  // `image[]` for multiple, `image` for one — OpenAI accepts both; use `image[]` uniformly.
  for img in req.images {
    let part = Part::bytes(img.bytes)
      .file_name(img.filename)
      .mime_str(&img.mime)
      .map_err(|e| OpenAiClientError::InvalidRequest(format!("bad image mime: {e}")))?;
    form = form.part("image[]", part);
  }
  if let Some(mask) = req.mask {
    let part = Part::bytes(mask.bytes)
      .file_name(mask.filename)
      .mime_str(&mask.mime)
      .map_err(|e| OpenAiClientError::InvalidRequest(format!("bad mask mime: {e}")))?;
    form = form.part("mask", part);
  }

  let client = client_core::http_client("foundation/openai-client");
  let builder = client.post(format!("{OPENAI_API_BASE_URL}/v1/images/edits")).multipart(form);

  let response = with_auth(builder, args.api_key)
    .send()
    .await
    .map_err(OpenAiGenericApiError::ReqwestError)?;

  let status = response.status();
  let text = response.text().await.map_err(OpenAiGenericApiError::ReqwestError)?;
  info!(%status, "openai image_edit response");

  classify_openai_http_error(status, Some(&text))?;

  let parsed: ImagesResponseBody = serde_json::from_str(&text)
    .map_err(|e| OpenAiGenericApiError::SerdeResponseParseErrorWithBody(e, text.clone()))?;

  let mime = OutputFormat::from_wire(parsed.output_format.as_deref().unwrap_or("png")).mime();
  let images = decode_images(parsed.data, mime)?;
  Ok(ImageGenerationSuccess { images, usage: parsed.usage.unwrap_or_default() })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn empty_images_rejected_client_side() {
    let key = OpenAiApiKey::new("sk-test");
    let err = image_edit(ImageEditArgs {
      api_key: &key,
      request: ImageEditRequest::new("make it winter", vec![]),
    })
    .await
    .unwrap_err();
    assert!(matches!(err, OpenAiError::Client(OpenAiClientError::InvalidRequest(_))));
  }

  #[test]
  fn debug_hides_image_bytes() {
    let req = ImageEditRequest::new("x", vec![InputImage::png(vec![1, 2, 3, 4], "in.png")]);
    let dbg = format!("{req:?}");
    assert!(dbg.contains("<4 bytes>"), "expected byte-count redaction, got {dbg}");
  }
}
