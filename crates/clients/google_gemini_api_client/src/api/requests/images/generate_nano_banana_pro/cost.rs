use client_core::{CostCalculator, CostCertainty, Micros};

use crate::api::requests::images::generate_nano_banana_pro::generate_nano_banana_pro::NanoBananaProRequest;
use crate::api::types::image_types::nano_banana_pro_image_size::NanoBananaProImageSize;

// Nano Banana Pro (gemini-3-pro-image): $0.134 (1K/2K, default 1K) / $0.24 (4K) per image.

impl CostCalculator for NanoBananaProRequest {
  fn cost_micros(&self) -> Micros {
    match self.image_size.unwrap_or(NanoBananaProImageSize::K1) {
    NanoBananaProImageSize::K1 | NanoBananaProImageSize::K2 => Micros(134_000),
    NanoBananaProImageSize::K4 => Micros(240_000),
  }
  }

  /// Thinking tokens are billed by default on top of the per-image tier — provider usage is authoritative.
  fn cost_certainty(&self) -> CostCertainty {
    CostCertainty::Estimated
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn req(size: Option<NanoBananaProImageSize>) -> NanoBananaProRequest {
    NanoBananaProRequest { prompt: "p".into(), images: vec![], aspect_ratio: None, image_size: size }
  }

  #[test]
  fn tiers_follow_image_size() {
    assert_eq!(req(None).cost_micros(), Micros(134_000)); // default 1K
    assert_eq!(req(Some(NanoBananaProImageSize::K1)).cost_micros(), Micros(134_000));
    assert_eq!(req(Some(NanoBananaProImageSize::K2)).cost_micros(), Micros(134_000));
    assert_eq!(req(Some(NanoBananaProImageSize::K4)).cost_micros(), Micros(240_000));
  }

  #[test]
  fn estimated_because_thinking_tokens_bill_on_top() {
    assert_eq!(req(None).cost_certainty(), CostCertainty::Estimated);
  }
}


use client_core::{BetterCostEstimator, CostBreakdown, CostComponent, CostFormula, CostQuantity, CostUnit, RateRow};

fn pro_tier_micros(size: NanoBananaProImageSize) -> i64 {
  match size {
    NanoBananaProImageSize::K1 | NanoBananaProImageSize::K2 => 134_000,
    NanoBananaProImageSize::K4 => 240_000,
  }
}

impl BetterCostEstimator for NanoBananaProRequest {
  fn estimate_cost(&self) -> CostBreakdown {
    let size = self.image_size.unwrap_or(NanoBananaProImageSize::K1);
    CostBreakdown::from_components(
      CostFormula {
        display: "1 image × tier(image_size) + thinking tokens",
        rates: [NanoBananaProImageSize::K1, NanoBananaProImageSize::K2, NanoBananaProImageSize::K4]
          .iter()
          .map(|s| RateRow {
            dimension: "image_size",
            value: s.as_str().to_string(),
            unit: CostUnit::PerImage,
            price_micros: pro_tier_micros(*s),
          })
          .collect(),
      },
      vec![
        CostComponent::new(
          format!("Output image ({} tier)", size.as_str()),
          CostUnit::PerImage,
          pro_tier_micros(size),
          CostQuantity::Exact(1.0),
        )
        .with_note("thinking tokens bill on top of the per-image tier; provider usage is authoritative"),
      ],
      true,
    )
  }
}

#[cfg(test)]
mod better_tests {
  use super::*;

  #[test]
  fn matches_legacy_totals() {
    for size in [None, Some(NanoBananaProImageSize::K2), Some(NanoBananaProImageSize::K4)] {
      let request =
        NanoBananaProRequest { prompt: "p".into(), images: vec![], aspect_ratio: None, image_size: size };
      assert_eq!(request.estimate_cost().amount.micros(), request.cost_micros());
    }
  }
}
