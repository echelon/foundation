use client_core::{CostCalculator, Micros};

use crate::api::requests::images::image_edit::image_edit::ImageEditRequest;
use crate::api::requests::images::per_image_micros;
use crate::api::types::image_types::image_model::ImageModel;
use crate::api::types::image_types::image_quality::ImageQuality;

/// Edit cost is the OUTPUT side, same per-image rates as generation, scaled by `n`.
///
/// NOTE: GPT Image also bills INPUT image tokens for edits, which depend on the input
/// image dimensions (not present in the request as pixel counts). This estimator covers
/// the dominant output cost only; the true bill from the response `usage` is authoritative.
impl CostCalculator for ImageEditRequest {
  fn cost_micros(&self) -> Micros {
    let model = self.model.clone().unwrap_or(ImageModel::GptImage1);
    let quality = self.quality.unwrap_or(ImageQuality::Medium);
    let n = self.number_images.unwrap_or(1) as i64;
    Micros(per_image_micros(&model, quality) * n)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::api::requests::images::image_edit::request_types::InputImage;

  fn req(model: Option<ImageModel>, quality: Option<ImageQuality>, n: Option<u32>) -> ImageEditRequest {
    ImageEditRequest {
      prompt: "t".into(),
      images: vec![InputImage::png(vec![0], "a.png")],
      mask: None,
      model,
      number_images: n,
      size: None,
      quality,
      output_format: None,
      user: None,
    }
  }

  #[test]
  fn matches_generation_output_rate() {
    assert_eq!(
      req(Some(ImageModel::GptImage1), Some(ImageQuality::High), Some(1)).cost_micros(),
      Micros(80_000)
    );
  }

  #[test]
  fn scales_with_n() {
    assert_eq!(
      req(Some(ImageModel::GptImage1_5), Some(ImageQuality::Medium), Some(3)).cost_micros(),
      Micros(30_000 * 3)
    );
  }

  #[test]
  fn cost_independent_of_input_image_count() {
    let one = req(Some(ImageModel::GptImage1), Some(ImageQuality::Medium), Some(1));
    let mut many = req(Some(ImageModel::GptImage1), Some(ImageQuality::Medium), Some(1));
    many.images = vec![InputImage::png(vec![0], "a.png"), InputImage::png(vec![0], "b.png")];
    assert_eq!(one.cost_micros(), many.cost_micros());
  }
}
