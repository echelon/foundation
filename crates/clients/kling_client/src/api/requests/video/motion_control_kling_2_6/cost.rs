use client_core::{CostBreakdown, CostCalculator, CostComponent, CostFormula, CostQuantity, CostUnit, RateRow};

use crate::api::requests::video::motion_control_kling_2_6::motion_control_kling_2_6::MotionControlKling26Request;
use crate::api::types::video_types::video_resolution::VideoResolution;

/// Official pricing (pricing/base/video): Kling 2.6 motion control bills per
/// output second — $0.07/s at 720p, $0.112/s at 1080p.
const PER_SECOND_720P_MICROS: i64 = 70_000;
const PER_SECOND_1080P_MICROS: i64 = 112_000;

/// The output duration follows the motion reference video, which the client
/// can't measure from a URL — estimates assume this many seconds.
const ASSUMED_OUTPUT_SECONDS: f64 = 10.0;

impl CostCalculator for MotionControlKling26Request {
  fn estimate_cost(&self) -> CostBreakdown {
    let resolution = self.resolution.unwrap_or(VideoResolution::P720);
    let per_second = match resolution {
      VideoResolution::P720 => PER_SECOND_720P_MICROS,
      VideoResolution::P1080 | VideoResolution::K4 => PER_SECOND_1080P_MICROS,
    };
    CostBreakdown::from_components(
      CostFormula {
        display: "output_seconds × rate(resolution); output duration follows the motion reference video",
        rates: [("720p", PER_SECOND_720P_MICROS), ("1080p", PER_SECOND_1080P_MICROS)]
          .iter()
          .map(|(value, price)| RateRow {
            dimension: "resolution",
            value: value.to_string(),
            unit: CostUnit::PerVideoSecond,
            price_micros: *price,
          })
          .collect(),
      },
      vec![CostComponent::new(
        format!("Video output (~{ASSUMED_OUTPUT_SECONDS}s assumed @ {})", resolution.as_str()),
        CostUnit::PerVideoSecond,
        per_second,
        CostQuantity::Estimated(ASSUMED_OUTPUT_SECONDS),
      )
      .with_note("billed on actual output duration (≈ reference video length, ≤30s); task billing is authoritative")],
      true,
    )
  }
}

#[cfg(test)]
mod tests {
  use client_core::{CostCertainty, MicroUsd};

  use super::*;
  use crate::api::types::video_types::character_orientation::CharacterOrientation;

  fn request(resolution: Option<VideoResolution>) -> MotionControlKling26Request {
    let mut r = MotionControlKling26Request::new(
      "https://example.com/p.png",
      "https://example.com/d.mp4",
      CharacterOrientation::Video,
    );
    r.resolution = resolution;
    r
  }

  #[test]
  fn published_rates_at_assumed_10s() {
    assert_eq!(request(None).estimate_cost().amount.micros(), MicroUsd(700_000));
    assert_eq!(
      request(Some(VideoResolution::P1080)).estimate_cost().amount.micros(),
      MicroUsd(1_120_000)
    );
  }

  #[test]
  fn cheaper_than_3_0_motion_control() {
    use crate::api::requests::video::motion_control_kling_3_0::motion_control_kling_3_0::MotionControlKling30Request;
    let mut kling_3_0 =
      MotionControlKling30Request::new("https://example.com/d.mp4", CharacterOrientation::Video);
    kling_3_0.image = Some("https://example.com/p.png".to_string());
    assert!(
      request(None).estimate_cost().amount.micros().0 < kling_3_0.estimate_cost().amount.micros().0
    );
  }

  #[test]
  fn estimated_certainty() {
    assert_eq!(request(None).estimate_cost().amount.certainty(), CostCertainty::Estimated);
  }
}
