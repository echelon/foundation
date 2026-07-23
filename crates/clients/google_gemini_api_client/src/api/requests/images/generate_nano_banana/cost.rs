
use crate::api::requests::images::generate_nano_banana::generate_nano_banana::NanoBananaRequest;

pub(crate) const NANO_BANANA_MICROS_PER_IMAGE: i64 = 39_000;

// Nano Banana (gemini-2.5-flash-image): $0.039 per image, flat (1024px, ~1,290 output tokens).




use client_core::{BetterCostEstimator, CostBreakdown, CostComponent, CostFormula, CostQuantity, CostUnit, RateRow};

impl BetterCostEstimator for NanoBananaRequest {
  fn estimate_cost(&self) -> CostBreakdown {
    CostBreakdown::from_components(
      CostFormula {
        display: "1 image × flat rate",
        rates: vec![RateRow {
          dimension: "—",
          value: "any".to_string(),
          unit: CostUnit::PerImage,
          price_micros: NANO_BANANA_MICROS_PER_IMAGE,
        }],
      },
      vec![CostComponent::new(
        "Output image (1024px, flat)",
        CostUnit::PerImage,
        NANO_BANANA_MICROS_PER_IMAGE,
        CostQuantity::Exact(1.0),
      )],
      false,
    )
  }
}

#[cfg(test)]
mod better_tests {
  use super::*;
  use client_core::{CostAmount, MicroUsd};

  #[test]
  fn breakdown_matches_legacy_and_is_exact() {
    let request = NanoBananaRequest { prompt: "p".into(), images: vec![], aspect_ratio: None };
    let breakdown = request.estimate_cost();
    assert_eq!(breakdown.amount, CostAmount::Exact(MicroUsd(39_000)));
    assert_eq!(breakdown.components.len(), 1);
    assert_eq!(breakdown.formula.rates.len(), 1);
  }
}
