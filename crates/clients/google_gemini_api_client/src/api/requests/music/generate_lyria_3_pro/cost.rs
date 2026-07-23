use client_core::{CostCalculator, CostCertainty, Micros};

use crate::api::requests::music::generate_lyria_3_pro::generate_lyria_3_pro::Lyria3ProRequest;

// Lyria 3 Pro (lyria-3-pro-preview): $0.08 per song (per request).

impl CostCalculator for Lyria3ProRequest {
  fn cost_micros(&self) -> Micros {
    Micros(80_000)
  }

  /// Flat per-request pricing — the params fully determine the charge.
  fn cost_certainty(&self) -> CostCertainty {
    CostCertainty::Exact
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn flat_per_song() {
    let request = Lyria3ProRequest { prompt: "p".into(), images: vec![], wav_output: false };
    assert_eq!(request.cost_micros(), Micros(80_000));
    assert_eq!(request.cost_certainty(), CostCertainty::Exact);
  }
}


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
    assert_eq!(breakdown.amount.micros(), request.cost_micros());
    assert_eq!(breakdown.amount.certainty(), CostCertainty::Exact);
  }
}
