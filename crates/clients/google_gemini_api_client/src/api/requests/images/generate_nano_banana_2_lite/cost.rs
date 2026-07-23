
use crate::api::requests::images::generate_nano_banana_2_lite::generate_nano_banana_2_lite::NanoBanana2LiteRequest;

// Nano Banana 2 Lite (gemini-3.1-flash-lite-image): ~$0.0336 per 1K image (1K only; $30.00 / 1M output tokens).




use client_core::{BetterCostEstimator, CostBreakdown, CostComponent, CostFormula, CostQuantity, CostUnit, RateRow};

impl BetterCostEstimator for NanoBanana2LiteRequest {
  fn estimate_cost(&self) -> CostBreakdown {
    CostBreakdown::from_components(
      CostFormula {
        display: "1 image × flat 1K rate (token-priced: $30/1M output tokens)",
        rates: vec![RateRow {
          dimension: "—",
          value: "1K".to_string(),
          unit: CostUnit::PerImage,
          price_micros: 33_600,
        }],
      },
      vec![
        CostComponent::new("Output image (1K only)", CostUnit::PerImage, 33_600, CostQuantity::Estimated(1.0))
          .with_note("token-priced; the per-image figure assumes a typical 1K image"),
      ],
      true,
    )
  }
}

#[cfg(test)]
mod better_tests {
  use super::*;
  use client_core::CostCertainty;

  #[test]
  fn flat_estimated() {
    let request = NanoBanana2LiteRequest { prompt: "p".into(), images: vec![], aspect_ratio: None };
    let breakdown = request.estimate_cost();
    assert_eq!(breakdown.amount.certainty(), CostCertainty::Estimated);
    assert!(breakdown.amount.micros().0 > 0);
  }
}
