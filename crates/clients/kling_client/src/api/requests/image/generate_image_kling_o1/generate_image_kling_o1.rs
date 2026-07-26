use crate::api::requests::image::common::{
  validate_n, ElementListEntry, ImageListEntry, OmniImageBody, MAX_IMAGES_PLUS_ELEMENTS, MAX_IMAGE_PROMPT_CHARS,
};
use crate::api::requests::legacy_common::legacy_post;
use crate::api::requests::video::common::validate_prompt;
use crate::api::types::image_types::image_aspect_ratio::ImageAspectRatio;
use crate::api::types::image_types::image_resolution::ImageResolution;
use crate::api::types::legacy_types::legacy_created_task::LegacyCreatedTask;
use crate::api::types::task_types::request_options::WatermarkInfo;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::kling_client_error::KlingClientError;
use crate::error::kling_error::KlingError;

pub(crate) const PATH: &str = "/v1/images/omni-image";
/// The wire model slug for Kling Image O1 (the endpoint's default model).
pub const KLING_IMAGE_O1_MODEL_NAME: &str = "kling-image-o1";

/// Kling Image O1 on `/v1/images/omni-image`: same reference-image/element
/// templating as 3.0 Omni, but 1K/2K only and no series generation.
#[derive(Clone, Debug)]
pub struct GenerateImageKlingO1Request {
  /// ≤2500 chars — positive AND negative description in one prompt;
  /// reference inputs via `<<<image_N>>>` / element placeholders.
  pub prompt: String,
  /// Reference images (URL or raw Base64, no `data:` prefix); with elements,
  /// ≤10 total.
  pub images: Vec<String>,
  /// Custom elements (Element Management API). Ids are numeric on this
  /// endpoint.
  pub element_ids: Vec<i64>,
  /// `1k` (default) or `2k` — O1 has no 4K tier.
  pub resolution: Option<ImageResolution>,
  /// Images per batch, 1–9 (default 1). Billing is per image.
  pub n: Option<u8>,
  /// Default `auto` (follow references).
  pub aspect_ratio: Option<ImageAspectRatio>,
  pub watermark: Option<WatermarkInfo>,
  /// Callbacks use the legacy dialect →
  /// [`crate::api::webhooks::parse_legacy_callback`].
  pub callback_url: Option<String>,
  pub external_task_id: Option<String>,
}

impl GenerateImageKlingO1Request {
  pub fn new(prompt: impl Into<String>) -> Self {
    Self {
      prompt: prompt.into(),
      images: vec![],
      element_ids: vec![],
      resolution: None,
      n: None,
      aspect_ratio: None,
      watermark: None,
      callback_url: None,
      external_task_id: None,
    }
  }
}

#[derive(Clone, Debug)]
pub struct GenerateImageKlingO1Args<'a> {
  pub api_key: &'a KlingApiKey,
  pub request: GenerateImageKlingO1Request,
}

pub(crate) fn build_body(request: &GenerateImageKlingO1Request) -> Result<OmniImageBody<'_>, KlingClientError> {
  validate_prompt(&request.prompt, MAX_IMAGE_PROMPT_CHARS)?;
  if request.resolution == Some(ImageResolution::R4k) {
    return Err(KlingClientError::InvalidRequest("kling-image-o1 supports 1k/2k only; 4k is kling-v3-omni-only".into()));
  }
  if let Some(n) = request.n {
    validate_n(n)?;
  }
  if request.images.len() + request.element_ids.len() > MAX_IMAGES_PLUS_ELEMENTS {
    return Err(KlingClientError::InvalidRequest(format!(
      "images + elements must be ≤ {MAX_IMAGES_PLUS_ELEMENTS}, got {}",
      request.images.len() + request.element_ids.len()
    )));
  }
  Ok(OmniImageBody {
    model_name: KLING_IMAGE_O1_MODEL_NAME,
    prompt: &request.prompt,
    image_list: request.images.iter().map(|image| ImageListEntry { image }).collect(),
    element_list: request.element_ids.iter().map(|id| ElementListEntry { element_id: *id }).collect(),
    resolution: request.resolution,
    // O1 is single-only; the field is left to the server default rather than
    // sent explicitly.
    result_type: None,
    n: request.n,
    series_amount: None,
    aspect_ratio: request.aspect_ratio,
    watermark_info: request.watermark,
    callback_url: request.callback_url.as_deref(),
    external_task_id: request.external_task_id.as_deref(),
  })
}

/// Dry-run: the exact JSON body [`generate_image_kling_o1`] would send.
pub fn generate_image_kling_o1_wire_json(request: &GenerateImageKlingO1Request) -> Result<serde_json::Value, KlingError> {
  let body = build_body(request)?;
  serde_json::to_value(&body).map_err(|e| KlingClientError::RequestSerializationError(e).into())
}

/// `POST /v1/images/omni-image` (model `kling-image-o1`) — async; poll with
/// [`crate::api::requests::image::get_omni_image_task`].
pub async fn generate_image_kling_o1(args: GenerateImageKlingO1Args<'_>) -> Result<LegacyCreatedTask, KlingError> {
  let GenerateImageKlingO1Args { api_key, request } = args;
  let body = build_body(&request)?;
  let body_json = serde_json::to_string(&body).map_err(KlingClientError::RequestSerializationError)?;
  legacy_post(api_key, PATH, body_json).await
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  // ── Wire-shape tests ──

  #[test]
  fn wire_shape_with_references() {
    let mut request = GenerateImageKlingO1Request::new("<<<image_1>>> as a watercolor painting");
    request.images = vec!["https://example.com/photo.png".to_string()];
    request.resolution = Some(ImageResolution::R2k);
    request.n = Some(2);
    assert_eq!(
      generate_image_kling_o1_wire_json(&request).unwrap(),
      json!({
        "model_name": "kling-image-o1",
        "prompt": "<<<image_1>>> as a watercolor painting",
        "image_list": [{"image": "https://example.com/photo.png"}],
        "resolution": "2k",
        "n": 2
      })
    );
  }

  #[test]
  fn wire_shape_minimal_never_emits_series_fields() {
    let request = GenerateImageKlingO1Request::new("a fox");
    let json = generate_image_kling_o1_wire_json(&request).unwrap();
    assert_eq!(json, json!({"model_name": "kling-image-o1", "prompt": "a fox"}));
  }

  // ── Validation tests ──

  #[test]
  fn rejects_4k() {
    let mut request = GenerateImageKlingO1Request::new("a fox");
    request.resolution = Some(ImageResolution::R4k);
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }

  #[test]
  fn rejects_out_of_range_n() {
    let mut request = GenerateImageKlingO1Request::new("a fox");
    request.n = Some(0);
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }
}
