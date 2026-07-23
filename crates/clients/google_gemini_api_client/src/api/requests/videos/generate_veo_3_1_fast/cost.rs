
use crate::api::requests::videos::generate_veo_3_1_fast::generate_veo_3_1_fast::Veo31FastRequest;
use crate::api::types::video_types::veo_duration::VeoDuration;
use crate::api::types::video_types::veo_resolution::VeoResolution;

// Veo 3.1 Fast (veo-3.1-fast-generate-preview): $0.10/s (720p) · $0.12/s (1080p) · $0.30/s (4K). Priced per second of
// output video; you are NOT charged for safety-blocked generations, so the
// per-params price is exact for every billed generation.




use client_core::{CostCalculator, CostBreakdown, CostComponent, CostFormula, CostQuantity, CostUnit, RateRow};

impl CostCalculator for Veo31FastRequest {
  fn estimate_cost(&self) -> CostBreakdown {
    let per_second: i64 = match self.resolution.unwrap_or(VeoResolution::P720) {
      VeoResolution::P720 => 100_000,
      VeoResolution::P1080 => 120_000,
      VeoResolution::K4 => 300_000,
    };
    let resolution = self.resolution.unwrap_or(VeoResolution::P720);
    let seconds = self.duration.unwrap_or(VeoDuration::S8).seconds();
    CostBreakdown::from_components(
      CostFormula {
        display: "seconds × rate(resolution); billed only on successful generation",
        rates: [("720p", 100_000), ("1080p", 120_000), ("4k", 300_000)]
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
    let request = Veo31FastRequest {
      prompt: "p".into(), negative_prompt: None, image: None, last_frame: None, reference_images: vec![],
      video_uri: None, person_generation: None, aspect_ratio: None, resolution: None, duration: None,
    };
    let breakdown = request.estimate_cost();
    assert_eq!(breakdown.amount.certainty(), CostCertainty::Exact);
    assert_eq!(breakdown.components[0].quantity, CostQuantity::Exact(8.0));
    assert!(breakdown.amount.micros().0 > 0);
  }
}
