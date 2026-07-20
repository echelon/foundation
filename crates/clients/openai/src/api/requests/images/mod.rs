pub mod image_edit;
pub mod image_generation;

/// Per-image output rate in micro-USD by (model, quality). Shared by generation + edit
/// (same output side). Published GPT Image rates as of these docs; Custom assumes the
/// pricier GPT Image 2 tier.
pub(crate) fn per_image_micros(
  model: &crate::api::types::image_types::image_model::ImageModel,
  quality: crate::api::types::image_types::image_quality::ImageQuality,
) -> i64 {
  use crate::api::types::image_types::image_model::ImageModel::*;
  use crate::api::types::image_types::image_quality::ImageQuality::*;
  // (low, medium, high); Auto is billed as medium.
  let (low, medium, high) = match model {
    GptImage1 => (20_000, 40_000, 80_000),
    GptImage1Mini => (10_000, 20_000, 40_000),
    GptImage1_5 => (15_000, 30_000, 60_000),
    GptImage2 => (40_000, 60_000, 120_000),
    Custom(_) => (40_000, 60_000, 120_000),
  };
  match quality {
    Low => low,
    Medium | Auto => medium,
    High => high,
  }
}
