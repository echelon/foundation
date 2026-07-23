
use crate::api::requests::images::generate_nano_banana_2::generate_nano_banana_2::NanoBanana2Request;
use crate::api::types::image_types::nano_banana_2_image_size::NanoBanana2ImageSize;

// Nano Banana 2 (gemini-3.1-flash-image): $0.045 (512px) / $0.067 (1K, default) / $0.101 (2K) / $0.151 (4K) per image.




use client_core::{BetterCostEstimator, CostBreakdown, CostComponent, CostFormula, CostQuantity, CostUnit, RateRow};
#[cfg(test)]
use client_core::MicroUsd;

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
    assert_eq!(breakdown.amount.micros(), MicroUsd(151_000));
    assert_eq!(breakdown.amount.certainty(), CostCertainty::Estimated);
    assert_eq!(breakdown.components[0].unit_price_micros, 151_000);
    assert_eq!(breakdown.formula.rates.len(), 4);
  }
}
