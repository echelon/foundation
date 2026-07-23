
use crate::api::requests::music::generate_lyria_3_pro::generate_lyria_3_pro::Lyria3ProRequest;

// Lyria 3 Pro (lyria-3-pro-preview): $0.08 per song (per request).




use client_core::{BetterCostEstimator, CostBreakdown, CostComponent, CostFormula, CostQuantity, CostUnit, RateRow};

impl BetterCostEstimator for Lyria3ProRequest {
  fn estimate_cost(&self) -> CostBreakdown {
    CostBreakdown::from_components(
      CostFormula {
        display: "1 song × flat rate",
        rates: vec![RateRow {
          dimension: "—",
          value: "per song".to_string(),
          unit: CostUnit::PerRequest,
          price_micros: 80_000,
        }],
      },
      vec![CostComponent::new("Generated song", CostUnit::PerRequest, 80_000, CostQuantity::Exact(1.0))],
      false,
    )
  }
}

#[cfg(test)]
mod better_tests {
  use super::*;
  use client_core::CostCertainty;

  #[test]
  fn matches_legacy_and_is_exact() {
    let request = Lyria3ProRequest { prompt: "p".into(), images: vec![], wav_output: false };
    let breakdown = request.estimate_cost();
    assert_eq!(breakdown.amount.certainty(), CostCertainty::Exact);
    assert!(breakdown.amount.micros().0 > 0);
  }
}
