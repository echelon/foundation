use serde::Deserialize;

use crate::api::types::legacy_types::legacy_image_result::LegacyImageResult;

/// The `task_result` payload of image generation tasks. `series_images` is
/// populated (alongside `result_type: "series"`) only for omni-image series
/// results.
#[derive(Clone, Debug, Deserialize)]
pub struct ImageTaskResult {
  #[serde(default)]
  pub images: Vec<LegacyImageResult>,
  #[serde(default)]
  pub series_images: Vec<LegacyImageResult>,
  #[serde(default)]
  pub result_type: Option<String>,
}
