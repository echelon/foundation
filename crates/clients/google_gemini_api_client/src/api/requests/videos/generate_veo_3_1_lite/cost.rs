use client_core::{CostCalculator, CostCertainty, Micros};

use crate::api::requests::videos::generate_veo_3_1_lite::generate_veo_3_1_lite::Veo31LiteRequest;
use crate::api::types::video_types::veo_duration::VeoDuration;
use crate::api::types::video_types::veo_lite_resolution::VeoLiteResolution;

// Veo 3.1 Lite (veo-3.1-lite-generate-preview): $0.05/s (720p) · $0.08/s (1080p); no 4K tier. Priced per second of
// output video; you are NOT charged for safety-blocked generations, so the
// per-params price is exact for every billed generation.

impl CostCalculator for Veo31LiteRequest {
  fn cost_micros(&self) -> Micros {
    let per_second = match self.resolution.unwrap_or(VeoLiteResolution::P720) {
    VeoLiteResolution::P720 => 50_000,
    VeoLiteResolution::P1080 => 80_000,
  };
    let seconds = self.duration.unwrap_or(VeoDuration::S8).seconds();
    Micros(per_second * seconds)
  }

  /// Exact per the params: Veo bills per output second, and safety-blocked
  /// generations are not charged at all.
  fn cost_certainty(&self) -> CostCertainty {
    CostCertainty::Exact
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn req(res: Option<VeoLiteResolution>, dur: Option<VeoDuration>) -> Veo31LiteRequest {
    Veo31LiteRequest { prompt: "p".into(), negative_prompt: None, image: None, last_frame: None, reference_images: vec![], video_uri: None, person_generation: None, aspect_ratio: None, resolution: res, duration: dur }
  }

  #[test]
  fn per_second_by_resolution() {
    assert_eq!(req(None, None).cost_micros(), Micros(400_000)); // 8s × $0.05 default
    assert_eq!(req(Some(VeoLiteResolution::P1080), Some(VeoDuration::S4)).cost_micros(), Micros(320_000));
    assert_eq!(req(None, None).cost_certainty(), CostCertainty::Exact);
  }
}


use client_core::{BetterCostEstimator, CostBreakdown, CostComponent, CostFormula, CostQuantity, CostUnit, RateRow};

impl BetterCostEstimator for Veo31LiteRequest {
  fn estimate_cost(&self) -> CostBreakdown {
    let per_second: i64 = match self.resolution.unwrap_or(VeoLiteResolution::P720) {
      VeoLiteResolution::P720 => 50_000,
      VeoLiteResolution::P1080 => 80_000,
    };
    let resolution = self.resolution.unwrap_or(VeoLiteResolution::P720);
    let seconds = self.duration.unwrap_or(VeoDuration::S8).seconds();
    CostBreakdown::from_components(
      CostFormula {
        display: "seconds × rate(resolution); billed only on successful generation",
        rates: [("720p", 50_000), ("1080p", 80_000)]
          .iter()
          .map(|(value, price)| RateRow {
            dimension: "resolution",
            value: value.to_string(),
            unit: CostUnit::PerVideoSecond,
            price_micros: *price,
          })
          .collect(),
      },
      vec![
        CostComponent::new(
          format!("Video output ({}s @ {})", seconds, resolution.as_str()),
          CostUnit::PerVideoSecond,
          per_second,
          CostQuantity::Exact(seconds as f64),
        )
        .with_note("audio included; safety-blocked generations are not charged"),
      ],
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
    let request = Veo31LiteRequest {
      prompt: "p".into(), negative_prompt: None, image: None, last_frame: None, reference_images: vec![],
      video_uri: None, person_generation: None, aspect_ratio: None, resolution: None, duration: None,
    };
    let breakdown = request.estimate_cost();
    assert_eq!(breakdown.amount.micros(), request.cost_micros());
    assert_eq!(breakdown.amount.certainty(), CostCertainty::Exact);
    assert_eq!(breakdown.components[0].quantity, CostQuantity::Exact(8.0));
  }
}
