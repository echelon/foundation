use client_core::{CostCalculator, CostCertainty, Micros};

use crate::api::requests::videos::generate_veo_3_1_fast::generate_veo_3_1_fast::Veo31FastRequest;
use crate::api::types::video_types::veo_duration::VeoDuration;
use crate::api::types::video_types::veo_resolution::VeoResolution;

// Veo 3.1 Fast (veo-3.1-fast-generate-preview): $0.10/s (720p) · $0.12/s (1080p) · $0.30/s (4K). Priced per second of
// output video; you are NOT charged for safety-blocked generations, so the
// per-params price is exact for every billed generation.

impl CostCalculator for Veo31FastRequest {
  fn cost_micros(&self) -> Micros {
    let per_second = match self.resolution.unwrap_or(VeoResolution::P720) {
    VeoResolution::P720 => 100_000,
    VeoResolution::P1080 => 120_000,
    VeoResolution::K4 => 300_000,
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

  fn req(res: Option<VeoResolution>, dur: Option<VeoDuration>) -> Veo31FastRequest {
    Veo31FastRequest { prompt: "p".into(), negative_prompt: None, image: None, aspect_ratio: None, resolution: res, duration: dur }
  }

  #[test]
  fn per_second_by_resolution() {
    assert_eq!(req(None, None).cost_micros(), Micros(800_000)); // 8s × $0.10 default
    assert_eq!(req(Some(VeoResolution::P1080), Some(VeoDuration::S8)).cost_micros(), Micros(960_000));
    assert_eq!(req(Some(VeoResolution::K4), Some(VeoDuration::S6)).cost_micros(), Micros(1_800_000));
    assert_eq!(req(None, None).cost_certainty(), CostCertainty::Exact);
  }
}
