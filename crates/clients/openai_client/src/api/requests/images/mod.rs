pub(crate) mod common;
pub mod generate_gpt_image_1;
pub mod generate_gpt_image_1_5;
pub mod generate_gpt_image_1_mini;
pub mod generate_gpt_image_2;

/// Per-image OUTPUT rate in micro-USD by (model, quality) at 1024×1024 — the published
/// July-2026 image rates (`Auto` bills as medium; `Custom` assumes the GPT Image 2 tier).
/// ESTIMATE ONLY: GPT Image is token-metered, so larger sizes and edit input images bill
/// more; the response `usage` is authoritative.
pub(crate) fn per_image_micros(
  model: &crate::api::types::image_types::image_model::ImageModel,
  quality: crate::api::types::image_types::image_quality::ImageQuality,
) -> i64 {
  use crate::api::types::image_types::image_model::ImageModel::*;
  use crate::api::types::image_types::image_quality::ImageQuality::*;
  // (low, medium, high) at 1024×1024.
  let (low, medium, high) = match model {
    GptImage1 => (11_000, 42_000, 167_000),     // $0.011 / $0.042 / $0.167
    GptImage1Mini => (5_000, 11_000, 36_000),   // $0.005 / $0.011 / $0.036
    GptImage1_5 => (9_000, 34_000, 133_000),    // $0.009 / $0.034 / $0.133
    GptImage2 => (6_000, 53_000, 211_000),      // $0.006 / $0.053 / $0.211
    Custom(_) => (6_000, 53_000, 211_000),
  };
  match quality {
    Low => low,
    Medium | Auto => medium,
    High => high,
  }
}
