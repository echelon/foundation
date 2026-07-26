use crate::api::requests::image::common::{
  validate_n, ElementListEntry, ImageListEntry, OmniImageBody, MAX_IMAGES_PLUS_ELEMENTS, MAX_IMAGE_PROMPT_CHARS,
};
use crate::api::requests::legacy_common::legacy_post;
use crate::api::requests::video::common::validate_prompt;
use crate::api::types::image_types::image_aspect_ratio::ImageAspectRatio;
use crate::api::types::image_types::image_resolution::ImageResolution;
use crate::api::types::image_types::series_amount::SeriesAmount;
use crate::api::types::legacy_types::legacy_created_task::LegacyCreatedTask;
use crate::api::types::task_types::request_options::WatermarkInfo;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::kling_client_error::KlingClientError;
use crate::error::kling_error::KlingError;

pub(crate) const PATH: &str = "/v1/images/omni-image";
/// The wire model slug for Kling Image 3.0 Omni on the omni-image endpoint.
pub const KLING_IMAGE_3_0_OMNI_MODEL_NAME: &str = "kling-v3-omni";

/// Single image(s) (`n` applies) or a coherent series (`series_amount`
/// applies; omni-only).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OmniImageResult {
  Single { n: Option<u8> },
  Series { amount: Option<SeriesAmount> },
}

/// Kling Image 3.0 Omni on `/v1/images/omni-image`: reference images and
/// elements templated into the prompt (`<<<image_1>>>`, `<<<object_1>>>`),
/// up to 4K, and series generation.
#[derive(Clone, Debug)]
pub struct OmniImageKling30OmniRequest {
  /// ≤2500 chars — positive AND negative description in one prompt;
  /// reference inputs via `<<<image_N>>>` / element placeholders.
  pub prompt: String,
  /// Reference images (URL or raw Base64, no `data:` prefix); with elements,
  /// ≤10 total.
  pub images: Vec<String>,
  /// Custom elements (Element Management API). Ids are numeric on this
  /// endpoint.
  pub element_ids: Vec<i64>,
  /// `1k` (default), `2k`, or `4k`.
  pub resolution: Option<ImageResolution>,
  pub result: OmniImageResult,
  /// Default `auto` (follow references).
  pub aspect_ratio: Option<ImageAspectRatio>,
  pub watermark: Option<WatermarkInfo>,
  /// Callbacks use the legacy dialect →
  /// [`crate::api::webhooks::parse_legacy_callback`].
  pub callback_url: Option<String>,
  pub external_task_id: Option<String>,
}

impl OmniImageKling30OmniRequest {
  pub fn new(prompt: impl Into<String>) -> Self {
    Self {
      prompt: prompt.into(),
      images: vec![],
      element_ids: vec![],
      resolution: None,
      result: OmniImageResult::Single { n: None },
      aspect_ratio: None,
      watermark: None,
      callback_url: None,
      external_task_id: None,
    }
  }
}

#[derive(Clone, Debug)]
pub struct OmniImageKling30OmniArgs<'a> {
  pub api_key: &'a KlingApiKey,
  pub request: OmniImageKling30OmniRequest,
}

pub(crate) fn build_body(request: &OmniImageKling30OmniRequest) -> Result<OmniImageBody<'_>, KlingClientError> {
  validate_prompt(&request.prompt, MAX_IMAGE_PROMPT_CHARS)?;
  if request.images.len() + request.element_ids.len() > MAX_IMAGES_PLUS_ELEMENTS {
    return Err(KlingClientError::InvalidRequest(format!(
      "images + elements must be ≤ {MAX_IMAGES_PLUS_ELEMENTS}, got {}",
      request.images.len() + request.element_ids.len()
    )));
  }
  let (result_type, n, series_amount) = match request.result {
    OmniImageResult::Single { n } => {
      if let Some(n) = n {
        validate_n(n)?;
      }
      (n.map(|_| "single"), n, None)
    }
    OmniImageResult::Series { amount } => {
      if let Some(SeriesAmount::Count(count)) = amount {
        if !(2..=9).contains(&count) {
          return Err(KlingClientError::InvalidRequest(format!("series_amount must be in 2..=9 or auto, got {count}")));
        }
      }
      (Some("series"), None, amount)
    }
  };
  Ok(OmniImageBody {
    model_name: KLING_IMAGE_3_0_OMNI_MODEL_NAME,
    prompt: &request.prompt,
    image_list: request.images.iter().map(|image| ImageListEntry { image }).collect(),
    element_list: request.element_ids.iter().map(|id| ElementListEntry { element_id: *id }).collect(),
    resolution: request.resolution,
    result_type,
    n,
    series_amount,
    aspect_ratio: request.aspect_ratio,
    watermark_info: request.watermark,
    callback_url: request.callback_url.as_deref(),
    external_task_id: request.external_task_id.as_deref(),
  })
}

/// Dry-run: the exact JSON body [`omni_image_kling_3_0_omni`] would send.
pub fn omni_image_kling_3_0_omni_wire_json(
  request: &OmniImageKling30OmniRequest,
) -> Result<serde_json::Value, KlingError> {
  let body = build_body(request)?;
  serde_json::to_value(&body).map_err(|e| KlingClientError::RequestSerializationError(e).into())
}

/// `POST /v1/images/omni-image` (model `kling-v3-omni`) — async; poll with
/// [`crate::api::requests::image::get_omni_image_task`].
pub async fn omni_image_kling_3_0_omni(args: OmniImageKling30OmniArgs<'_>) -> Result<LegacyCreatedTask, KlingError> {
  let OmniImageKling30OmniArgs { api_key, request } = args;
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
  fn wire_shape_references_and_elements() {
    let mut request = OmniImageKling30OmniRequest::new("<<<object_1>>> wearing <<<image_1>>>'s jacket");
    request.images = vec!["https://example.com/jacket.png".to_string()];
    request.element_ids = vec![829836802793406551];
    request.resolution = Some(ImageResolution::R4k);
    request.result = OmniImageResult::Single { n: Some(2) };
    request.aspect_ratio = Some(ImageAspectRatio::Auto);
    assert_eq!(
      omni_image_kling_3_0_omni_wire_json(&request).unwrap(),
      json!({
        "model_name": "kling-v3-omni",
        "prompt": "<<<object_1>>> wearing <<<image_1>>>'s jacket",
        "image_list": [{"image": "https://example.com/jacket.png"}],
        "element_list": [{"element_id": 829836802793406551_i64}],
        "resolution": "4k",
        "result_type": "single",
        "n": 2,
        "aspect_ratio": "auto"
      })
    );
  }

  #[test]
  fn wire_shape_series_with_auto_amount() {
    let mut request = OmniImageKling30OmniRequest::new("a fox in four seasons");
    request.result = OmniImageResult::Series { amount: Some(SeriesAmount::Auto) };
    assert_eq!(
      omni_image_kling_3_0_omni_wire_json(&request).unwrap(),
      json!({
        "model_name": "kling-v3-omni",
        "prompt": "a fox in four seasons",
        "result_type": "series",
        "series_amount": "auto"
      })
    );
  }

  // ── Validation tests ──

  #[test]
  fn rejects_series_amount_below_2() {
    let mut request = OmniImageKling30OmniRequest::new("a fox");
    request.result = OmniImageResult::Series { amount: Some(SeriesAmount::Count(1)) };
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }

  #[test]
  fn rejects_too_many_images_plus_elements() {
    let mut request = OmniImageKling30OmniRequest::new("a fox");
    request.images = (0..8).map(|i| format!("https://example.com/{i}.png")).collect();
    request.element_ids = vec![1, 2, 3];
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }
}
