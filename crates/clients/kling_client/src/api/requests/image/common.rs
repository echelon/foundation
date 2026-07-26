use serde::Serialize;

use crate::api::types::image_types::image_aspect_ratio::ImageAspectRatio;
use crate::api::types::image_types::image_resolution::ImageResolution;
use crate::api::types::image_types::series_amount::SeriesAmount;
use crate::api::types::task_types::request_options::WatermarkInfo;
use crate::error::kling_client_error::KlingClientError;

pub(crate) const MAX_IMAGE_PROMPT_CHARS: usize = 2500;
pub(crate) const MAX_IMAGES_PLUS_ELEMENTS: usize = 10;

/// The outbound body for `/v1/images/omni-image` (Kling Image 3.0 Omni and
/// Image O1 share it; `model_name` selects the model).
#[derive(Debug, Serialize)]
pub(crate) struct OmniImageBody<'a> {
  pub model_name: &'static str,
  pub prompt: &'a str,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub image_list: Vec<ImageListEntry<'a>>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub element_list: Vec<ElementListEntry>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub resolution: Option<ImageResolution>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub result_type: Option<&'static str>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub n: Option<u8>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub series_amount: Option<SeriesAmount>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub aspect_ratio: Option<ImageAspectRatio>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub watermark_info: Option<WatermarkInfo>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub callback_url: Option<&'a str>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub external_task_id: Option<&'a str>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ImageListEntry<'a> {
  pub image: &'a str,
}

/// Element references go out as `{"element_id": <number>}` — the image
/// endpoints type element ids as longs (unlike the video `contents`, which
/// take strings).
#[derive(Debug, Serialize)]
pub(crate) struct ElementListEntry {
  pub element_id: i64,
}

pub(crate) fn validate_n(n: u8) -> Result<(), KlingClientError> {
  if n == 0 || n > 9 {
    return Err(KlingClientError::InvalidRequest(format!("n must be in 1..=9, got {n}")));
  }
  Ok(())
}
