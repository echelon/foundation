use client_core::{CostCalculator, CostCertainty, MicroUsd};

use crate::api::requests::music::generate_lyria_3_clip::generate_lyria_3_clip::Lyria3ClipRequest;

// Lyria 3 Clip (lyria-3-clip-preview): $0.04 per song (per request).

impl CostCalculator for Lyria3ClipRequest {
  fn cost_micros(&self) -> MicroUsd {
    MicroUsd(40_000)
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
    let request = Lyria3ClipRequest { prompt: "p".into(), images: vec![] };
    assert_eq!(request.cost_micros(), MicroUsd(40_000));
    assert_eq!(request.cost_certainty(), CostCertainty::Exact);
  }
}


use client_core::{BetterCostEstimator, CostBreakdown, CostComponent, CostFormula, CostQuantity, CostUnit, RateRow};

impl BetterCostEstimator for Lyria3ClipRequest {
  fn estimate_cost(&self) -> CostBreakdown {
    CostBreakdown::from_components(
      CostFormula {
        display: "1 song × flat rate",
        rates: vec![RateRow {
          dimension: "—",
          value: "per song".to_string(),
          unit: CostUnit::PerRequest,
          price_micros: 40_000,
        }],
      },
      vec![CostComponent::new("Generated song", CostUnit::PerRequest, 40_000, CostQuantity::Exact(1.0))],
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
    let request = Lyria3ClipRequest { prompt: "p".into(), images: vec![] };
    let breakdown = request.estimate_cost();
    assert_eq!(breakdown.amount.micros(), request.cost_micros());
    assert_eq!(breakdown.amount.certainty(), CostCertainty::Exact);
  }
}
