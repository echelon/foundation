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
    Veo31LiteRequest { prompt: "p".into(), negative_prompt: None, image: None, aspect_ratio: None, resolution: res, duration: dur }
  }

  #[test]
  fn per_second_by_resolution() {
    assert_eq!(req(None, None).cost_micros(), Micros(400_000)); // 8s × $0.05 default
    assert_eq!(req(Some(VeoLiteResolution::P1080), Some(VeoDuration::S4)).cost_micros(), Micros(320_000));
    assert_eq!(req(None, None).cost_certainty(), CostCertainty::Exact);
  }
}
