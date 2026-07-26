use serde::Serialize;

use crate::api::requests::image::common::{
  validate_n, ElementListEntry, MAX_IMAGES_PLUS_ELEMENTS, MAX_IMAGE_PROMPT_CHARS,
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

pub(crate) const PATH: &str = "/v1/images/generations";
/// The wire model slug for Kling Image 3.0 on the generations endpoint.
pub const KLING_IMAGE_3_0_MODEL_NAME: &str = "kling-v3";

/// Kling Image 3.0 text-to-image / image-to-image on
/// `/v1/images/generations`. 1K/2K only (Omni has the 4K tier); poll with
/// [`crate::api::requests::image::get_image_task`].
#[derive(Clone, Debug)]
pub struct GenerateImageKling30Request {
  /// ≤2500 chars.
  pub prompt: String,
  /// ≤2500 chars. Not supported together with `image` (img2img).
  pub negative_prompt: Option<String>,
  /// Image-to-image reference: URL or raw Base64 (no `data:` prefix);
  /// `.jpg/.jpeg/.png`, ≤10MB, ≥300px, aspect 1:2.5–2.5:1.
  pub image: Option<String>,
  /// Custom elements (Element Management API); elements + reference images
  /// ≤ 10 total. Ids are numeric on this endpoint.
  pub element_ids: Vec<i64>,
  /// `1k` (default) or `2k`.
  pub resolution: Option<ImageResolution>,
  /// Images per batch, 1–9 (default 1). Billing is per image.
  pub n: Option<u8>,
  /// Default 16:9. `auto` is omni-only — rejected here.
  pub aspect_ratio: Option<ImageAspectRatio>,
  pub watermark: Option<WatermarkInfo>,
  /// Callbacks use the legacy dialect →
  /// [`crate::api::webhooks::parse_legacy_callback`].
  pub callback_url: Option<String>,
  pub external_task_id: Option<String>,
}

impl GenerateImageKling30Request {
  pub fn new(prompt: impl Into<String>) -> Self {
    Self {
      prompt: prompt.into(),
      negative_prompt: None,
      image: None,
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
pub struct GenerateImageKling30Args<'a> {
  pub api_key: &'a KlingApiKey,
  pub request: GenerateImageKling30Request,
}

#[derive(Serialize)]
pub(crate) struct GenerationsBody<'a> {
  model_name: &'static str,
  prompt: &'a str,
  #[serde(skip_serializing_if = "Option::is_none")]
  negative_prompt: Option<&'a str>,
  #[serde(skip_serializing_if = "Option::is_none")]
  image: Option<&'a str>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  element_list: Vec<ElementListEntry>,
  #[serde(skip_serializing_if = "Option::is_none")]
  resolution: Option<ImageResolution>,
  #[serde(skip_serializing_if = "Option::is_none")]
  n: Option<u8>,
  #[serde(skip_serializing_if = "Option::is_none")]
  aspect_ratio: Option<ImageAspectRatio>,
  #[serde(skip_serializing_if = "Option::is_none")]
  watermark_info: Option<WatermarkInfo>,
  #[serde(skip_serializing_if = "Option::is_none")]
  callback_url: Option<&'a str>,
  #[serde(skip_serializing_if = "Option::is_none")]
  external_task_id: Option<&'a str>,
}

pub(crate) fn build_body(request: &GenerateImageKling30Request) -> Result<GenerationsBody<'_>, KlingClientError> {
  validate_prompt(&request.prompt, MAX_IMAGE_PROMPT_CHARS)?;
  if let Some(negative_prompt) = &request.negative_prompt {
    if request.image.is_some() {
      return Err(KlingClientError::InvalidRequest("negative_prompt is not supported with an image reference".into()));
    }
    let chars = negative_prompt.chars().count();
    if chars > MAX_IMAGE_PROMPT_CHARS {
      return Err(KlingClientError::InvalidRequest(format!(
        "negative_prompt is {chars} chars; at most {MAX_IMAGE_PROMPT_CHARS} allowed"
      )));
    }
  }
  if request.resolution == Some(ImageResolution::R4k) {
    return Err(KlingClientError::InvalidRequest("kling image 3.0 supports 1k/2k only; 4k is 3.0-omni-only".into()));
  }
  if request.aspect_ratio == Some(ImageAspectRatio::Auto) {
    return Err(KlingClientError::InvalidRequest("aspect_ratio auto is omni-image-only".into()));
  }
  if let Some(n) = request.n {
    validate_n(n)?;
  }
  let reference_images = usize::from(request.image.is_some());
  if request.element_ids.len() + reference_images > MAX_IMAGES_PLUS_ELEMENTS {
    return Err(KlingClientError::InvalidRequest(format!(
      "elements + reference images must be ≤ {MAX_IMAGES_PLUS_ELEMENTS}"
    )));
  }
  Ok(GenerationsBody {
    model_name: KLING_IMAGE_3_0_MODEL_NAME,
    prompt: &request.prompt,
    negative_prompt: request.negative_prompt.as_deref(),
    image: request.image.as_deref(),
    element_list: request.element_ids.iter().map(|id| ElementListEntry { element_id: *id }).collect(),
    resolution: request.resolution,
    n: request.n,
    aspect_ratio: request.aspect_ratio,
    watermark_info: request.watermark,
    callback_url: request.callback_url.as_deref(),
    external_task_id: request.external_task_id.as_deref(),
  })
}

/// Dry-run: the exact JSON body [`generate_image_kling_3_0`] would send.
pub fn generate_image_kling_3_0_wire_json(request: &GenerateImageKling30Request) -> Result<serde_json::Value, KlingError> {
  let body = build_body(request)?;
  serde_json::to_value(&body).map_err(|e| KlingClientError::RequestSerializationError(e).into())
}

/// `POST /v1/images/generations` (model `kling-v3`) — async; poll with
/// [`crate::api::requests::image::get_image_task`].
pub async fn generate_image_kling_3_0(args: GenerateImageKling30Args<'_>) -> Result<LegacyCreatedTask, KlingError> {
  let GenerateImageKling30Args { api_key, request } = args;
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
  fn wire_shape_text_to_image() {
    let mut request = GenerateImageKling30Request::new("a red fox in the snow");
    request.negative_prompt = Some("blurry".to_string());
    request.resolution = Some(ImageResolution::R2k);
    request.n = Some(4);
    request.aspect_ratio = Some(ImageAspectRatio::Portrait2x3);
    request.watermark = Some(WatermarkInfo { enabled: false });
    request.external_task_id = Some("job_01ABC".to_string());
    assert_eq!(
      generate_image_kling_3_0_wire_json(&request).unwrap(),
      json!({
        "model_name": "kling-v3",
        "prompt": "a red fox in the snow",
        "negative_prompt": "blurry",
        "resolution": "2k",
        "n": 4,
        "aspect_ratio": "2:3",
        "watermark_info": {"enabled": false},
        "external_task_id": "job_01ABC"
      })
    );
  }

  #[test]
  fn wire_shape_image_to_image_with_elements() {
    let mut request = GenerateImageKling30Request::new("same character, now at night");
    request.image = Some("https://example.com/ref.png".to_string());
    request.element_ids = vec![829836802793406551];
    assert_eq!(
      generate_image_kling_3_0_wire_json(&request).unwrap(),
      json!({
        "model_name": "kling-v3",
        "prompt": "same character, now at night",
        "image": "https://example.com/ref.png",
        "element_list": [{"element_id": 829836802793406551_i64}]
      })
    );
  }

  // ── Validation tests ──

  #[test]
  fn negative_prompt_conflicts_with_image_reference() {
    let mut request = GenerateImageKling30Request::new("a fox");
    request.image = Some("https://example.com/r.png".to_string());
    request.negative_prompt = Some("blurry".to_string());
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }

  #[test]
  fn rejects_4k_and_auto_aspect() {
    let mut four_k = GenerateImageKling30Request::new("a fox");
    four_k.resolution = Some(ImageResolution::R4k);
    assert!(matches!(build_body(&four_k), Err(KlingClientError::InvalidRequest(_))));

    let mut auto = GenerateImageKling30Request::new("a fox");
    auto.aspect_ratio = Some(ImageAspectRatio::Auto);
    assert!(matches!(build_body(&auto), Err(KlingClientError::InvalidRequest(_))));
  }

  #[test]
  fn rejects_out_of_range_n() {
    let mut request = GenerateImageKling30Request::new("a fox");
    request.n = Some(10);
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }
}
