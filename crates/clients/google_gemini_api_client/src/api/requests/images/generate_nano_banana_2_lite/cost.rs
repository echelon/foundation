use client_core::{CostCalculator, CostCertainty, Micros};

use crate::api::requests::images::generate_nano_banana_2_lite::generate_nano_banana_2_lite::NanoBanana2LiteRequest;

// Nano Banana 2 Lite (gemini-3.1-flash-lite-image): ~$0.0336 per 1K image (1K only; $30.00 / 1M output tokens).

impl CostCalculator for NanoBanana2LiteRequest {
  fn cost_micros(&self) -> Micros {
    Micros(33_600)
  }

  /// Token-priced ($30/1M) — the per-image figure assumes a typical 1K image; provider usage is authoritative.
  fn cost_certainty(&self) -> CostCertainty {
    CostCertainty::Estimated
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn req() -> NanoBanana2LiteRequest {
    NanoBanana2LiteRequest { prompt: "p".into(), images: vec![], aspect_ratio: None }
  }

  #[test]
  fn flat_1k_only() {
    assert_eq!(req().cost_micros(), Micros(33_600)); // ~$0.0336
    assert_eq!(req().cost_certainty(), CostCertainty::Estimated); // token-priced
  }
}


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
    assert_eq!(breakdown.amount.micros(), request.cost_micros());
    assert_eq!(breakdown.amount.certainty(), CostCertainty::Estimated);
  }
}
