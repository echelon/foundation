use client_core::{CostBreakdown, CostCalculator, CostComponent, CostFormula, CostQuantity, CostUnit, RateRow};

use crate::api::requests::image::generate_image_kling_o1::generate_image_kling_o1::GenerateImageKlingO1Request;

/// Official pricing (pricing/base/image): Kling Image O1 is $0.028 per image
/// at 1K/2K (no 4K tier). Billed per image, so `n` multiplies.
const PER_IMAGE_MICROS: i64 = 28_000;

impl CostCalculator for GenerateImageKlingO1Request {
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

  #[test]
  fn published_rate_and_n_scaling() {
    let mut request = GenerateImageKlingO1Request::new("a fox");
    assert_eq!(request.estimate_cost().amount.micros(), MicroUsd(28_000));
    request.n = Some(5);
    assert_eq!(request.estimate_cost().amount.micros(), MicroUsd(140_000));
    assert_eq!(request.estimate_cost().amount.certainty(), CostCertainty::Exact);
  }

  #[test]
  fn same_rate_as_image_3_0() {
    use crate::api::requests::image::generate_image_kling_3_0::generate_image_kling_3_0::GenerateImageKling30Request;
    let o1 = GenerateImageKlingO1Request::new("a fox").estimate_cost().amount.micros();
    let v3 = GenerateImageKling30Request::new("a fox").estimate_cost().amount.micros();
    assert_eq!(o1, v3);
  }
}
