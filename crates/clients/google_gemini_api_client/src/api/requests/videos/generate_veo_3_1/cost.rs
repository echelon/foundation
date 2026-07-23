use client_core::{CostCalculator, CostCertainty, MicroUsd};

use crate::api::requests::videos::generate_veo_3_1::generate_veo_3_1::Veo31Request;
use crate::api::types::video_types::veo_duration::VeoDuration;
use crate::api::types::video_types::veo_resolution::VeoResolution;

// Veo 3.1 (veo-3.1-generate-preview): $0.40/s (720p/1080p) · $0.60/s (4K), audio included. Priced per second of
// output video; you are NOT charged for safety-blocked generations, so the
// per-params price is exact for every billed generation.

impl CostCalculator for Veo31Request {
  fn cost_micros(&self) -> MicroUsd {
    let per_second = match self.resolution.unwrap_or(VeoResolution::P720) {
    VeoResolution::P720 | VeoResolution::P1080 => 400_000,
    VeoResolution::K4 => 600_000,
  };
    let seconds = self.duration.unwrap_or(VeoDuration::S8).seconds();
    MicroUsd(per_second * seconds)
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

  fn req(res: Option<VeoResolution>, dur: Option<VeoDuration>) -> Veo31Request {
    Veo31Request { prompt: "p".into(), negative_prompt: None, image: None, last_frame: None, reference_images: vec![], video_uri: None, person_generation: None, aspect_ratio: None, resolution: res, duration: dur }
  }

  #[test]
  fn per_second_by_resolution() {
    assert_eq!(req(None, None).cost_micros(), MicroUsd(3_200_000)); // 8s × $0.40 default
    assert_eq!(req(Some(VeoResolution::P1080), Some(VeoDuration::S4)).cost_micros(), MicroUsd(1_600_000));
    assert_eq!(req(Some(VeoResolution::K4), Some(VeoDuration::S8)).cost_micros(), MicroUsd(4_800_000));
    assert_eq!(req(None, None).cost_certainty(), CostCertainty::Exact);
  }
}


use client_core::{BetterCostEstimator, CostBreakdown, CostComponent, CostFormula, CostQuantity, CostUnit, RateRow};

impl BetterCostEstimator for Veo31Request {
  fn estimate_cost(&self) -> CostBreakdown {
    let per_second: i64 = match self.resolution.unwrap_or(VeoResolution::P720) {
      VeoResolution::P720 | VeoResolution::P1080 => 400_000,
      VeoResolution::K4 => 600_000,
    };
    let resolution = self.resolution.unwrap_or(VeoResolution::P720);
    let seconds = self.duration.unwrap_or(VeoDuration::S8).seconds();
    CostBreakdown::from_components(
      CostFormula {
        display: "seconds × rate(resolution); billed only on successful generation",
        rates: [("720p", 400_000), ("1080p", 400_000), ("4k", 600_000)]
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
    let request = Veo31Request {
      prompt: "p".into(), negative_prompt: None, image: None, last_frame: None, reference_images: vec![],
      video_uri: None, person_generation: None, aspect_ratio: None, resolution: None, duration: None,
    };
    let breakdown = request.estimate_cost();
    assert_eq!(breakdown.amount.micros(), request.cost_micros());
    assert_eq!(breakdown.amount.certainty(), CostCertainty::Exact);
    assert_eq!(breakdown.components[0].quantity, CostQuantity::Exact(8.0));
  }
}
