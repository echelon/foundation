use client_core::{CostCalculator, CostCertainty, Micros};

use crate::api::requests::images::generate_nano_banana_2::generate_nano_banana_2::NanoBanana2Request;
use crate::api::types::image_types::nano_banana_2_image_size::NanoBanana2ImageSize;

// Nano Banana 2 (gemini-3.1-flash-image): $0.045 (512px) / $0.067 (1K, default) / $0.101 (2K) / $0.151 (4K) per image.

impl CostCalculator for NanoBanana2Request {
  fn cost_micros(&self) -> Micros {
    match self.image_size.unwrap_or(NanoBanana2ImageSize::K1) {
    NanoBanana2ImageSize::P512 => Micros(45_000),
    NanoBanana2ImageSize::K1 => Micros(67_000),
    NanoBanana2ImageSize::K2 => Micros(101_000),
    NanoBanana2ImageSize::K4 => Micros(151_000),
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

  fn req(size: Option<NanoBanana2ImageSize>) -> NanoBanana2Request {
    NanoBanana2Request { prompt: "p".into(), images: vec![], aspect_ratio: None, image_size: size }
  }

  #[test]
  fn tiers_follow_image_size() {
    assert_eq!(req(None).cost_micros(), Micros(67_000)); // default 1K
    assert_eq!(req(Some(NanoBanana2ImageSize::P512)).cost_micros(), Micros(45_000));
    assert_eq!(req(Some(NanoBanana2ImageSize::K1)).cost_micros(), Micros(67_000));
    assert_eq!(req(Some(NanoBanana2ImageSize::K2)).cost_micros(), Micros(101_000));
    assert_eq!(req(Some(NanoBanana2ImageSize::K4)).cost_micros(), Micros(151_000));
  }

  #[test]
  fn estimated_because_thinking_tokens_bill_on_top() {
    assert_eq!(req(None).cost_certainty(), CostCertainty::Estimated);
  }
}


use client_core::{BetterCostEstimator, CostBreakdown, CostComponent, CostFormula, CostQuantity, CostUnit, RateRow};

const NB2_THINKING_NOTE: &str = "thinking tokens bill on top of the per-image tier; provider usage is authoritative";

fn nb2_tier_micros(size: NanoBanana2ImageSize) -> i64 {
  match size {
    NanoBanana2ImageSize::P512 => 45_000,
    NanoBanana2ImageSize::K1 => 67_000,
    NanoBanana2ImageSize::K2 => 101_000,
    NanoBanana2ImageSize::K4 => 151_000,
  }
}

impl BetterCostEstimator for NanoBanana2Request {
  fn estimate_cost(&self) -> CostBreakdown {
    let size = self.image_size.unwrap_or(NanoBanana2ImageSize::K1);
    CostBreakdown::from_components(
      CostFormula {
        display: "1 image × tier(image_size) + thinking tokens",
        rates: [
          NanoBanana2ImageSize::P512,
          NanoBanana2ImageSize::K1,
          NanoBanana2ImageSize::K2,
          NanoBanana2ImageSize::K4,
        ]
        .iter()
        .map(|s| RateRow {
          dimension: "image_size",
          value: s.as_str().to_string(),
          unit: CostUnit::PerImage,
          price_micros: nb2_tier_micros(*s),
        })
        .collect(),
      },
      vec![
        CostComponent::new(
          format!("Output image ({} tier)", size.as_str()),
          CostUnit::PerImage,
          nb2_tier_micros(size),
          CostQuantity::Exact(1.0),
        )
        .with_note(NB2_THINKING_NOTE),
      ],
      true, // thinking tokens make the total an estimate even with a fixed tier
    )
  }
}

#[cfg(test)]
mod better_tests {
  use super::*;
  use client_core::CostCertainty;

  #[test]
  fn tier_selects_component_price_and_stays_estimated() {
    let request = NanoBanana2Request {
      prompt: "p".into(),
      images: vec![],
      aspect_ratio: None,
      image_size: Some(NanoBanana2ImageSize::K4),
    };
    let breakdown = request.estimate_cost();
    assert_eq!(breakdown.amount.micros(), request.cost_micros());
    assert_eq!(breakdown.amount.certainty(), CostCertainty::Estimated);
    assert_eq!(breakdown.components[0].unit_price_micros, 151_000);
    assert_eq!(breakdown.formula.rates.len(), 4);
  }
}
