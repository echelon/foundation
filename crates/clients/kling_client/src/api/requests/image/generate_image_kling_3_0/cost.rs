use client_core::{CostBreakdown, CostCalculator, CostComponent, CostFormula, CostQuantity, CostUnit, RateRow};

use crate::api::requests::image::generate_image_kling_3_0::generate_image_kling_3_0::GenerateImageKling30Request;

/// Official pricing (pricing/base/image): Kling Image 3.0 is 8 units —
/// $0.028 per image at 1K and 2K alike. Billed per image, so `n` multiplies.
pub(crate) const PER_IMAGE_MICROS: i64 = 28_000;

impl CostCalculator for GenerateImageKling30Request {
  fn estimate_cost(&self) -> CostBreakdown {
    let n = self.n.unwrap_or(1);
    CostBreakdown::from_components(
      CostFormula {
        display: "n × rate; 1k and 2k share the rate",
        rates: vec![RateRow {
          dimension: "resolution",
          value: "1k/2k".to_string(),
          unit: CostUnit::PerImage,
          price_micros: PER_IMAGE_MICROS,
        }],
      },
      vec![CostComponent::new(
        format!("Image output ({n} image(s))"),
        CostUnit::PerImage,
        PER_IMAGE_MICROS,
        CostQuantity::Exact(n as f64),
      )],
      false,
    )
  }
}

#[cfg(test)]
mod tests {
  use client_core::{CostCertainty, MicroUsd};

  use super::*;
  use crate::api::types::image_types::image_resolution::ImageResolution;

  fn request(n: Option<u8>) -> GenerateImageKling30Request {
    let mut r = GenerateImageKling30Request::new("a fox");
    r.n = n;
    r
  }

  #[test]
  fn published_rate_single_image() {
    assert_eq!(request(None).estimate_cost().amount.micros(), MicroUsd(28_000));
  }

  #[test]
  fn n_multiplies_the_bill() {
    assert_eq!(request(Some(9)).estimate_cost().amount.micros(), MicroUsd(9 * 28_000));
  }

  #[test]
  fn resolution_does_not_change_the_rate() {
    let mut two_k = request(None);
    two_k.resolution = Some(ImageResolution::R2k);
    assert_eq!(two_k.estimate_cost().amount.micros(), request(None).estimate_cost().amount.micros());
  }

  #[test]
  fn exact_certainty() {
    assert_eq!(request(None).estimate_cost().amount.certainty(), CostCertainty::Exact);
  }
}
