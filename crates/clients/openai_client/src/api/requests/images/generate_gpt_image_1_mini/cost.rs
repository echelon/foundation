use client_core::{CostCalculator, Micros};

use crate::api::requests::images::per_image_micros;
use crate::api::requests::images::generate_gpt_image_1_mini::generate_gpt_image_1_mini::GptImage1MiniRequest;
use crate::api::types::image_types::image_model::ImageModel;
use crate::api::types::image_types::image_quality::ImageQuality;

/// ESTIMATE, pinned to gpt-image-1-mini's published per-image rates at 1024×1024 (`Auto` bills as
/// medium). GPT Image is token-metered underneath: larger sizes and edit INPUT images bill
/// additional tokens not visible in this request, and the tool's LLM driver adds a sliver.
/// The response `usage` is authoritative post-call.
impl CostCalculator for GptImage1MiniRequest {
  fn cost_micros(&self) -> Micros {
    let quality = self.quality.unwrap_or(ImageQuality::Medium);
    Micros(per_image_micros(&ImageModel::GptImage1Mini, quality))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn req(quality: Option<ImageQuality>) -> GptImage1MiniRequest {
    let mut r = GptImage1MiniRequest::new("t");
    r.quality = quality;
    r
  }

  #[test]
  fn published_rates_at_1024() {
    assert_eq!(req(Some(ImageQuality::Low)).cost_micros(), Micros(5000));
    assert_eq!(req(Some(ImageQuality::Medium)).cost_micros(), Micros(11000));
    assert_eq!(req(Some(ImageQuality::High)).cost_micros(), Micros(36000));
  }

  #[test]
  fn default_and_auto_bill_as_medium() {
    assert_eq!(req(None).cost_micros(), Micros(11000));
    assert_eq!(req(Some(ImageQuality::Auto)).cost_micros(), Micros(11000));
  }

  #[test]
  fn quality_ordering_holds() {
    assert!(req(Some(ImageQuality::Low)).cost_micros() < req(Some(ImageQuality::Medium)).cost_micros());
    assert!(req(Some(ImageQuality::Medium)).cost_micros() < req(Some(ImageQuality::High)).cost_micros());
  }
}
