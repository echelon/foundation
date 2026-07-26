use client_core::{CostBreakdown, CostCalculator, CostComponent, CostFormula, CostQuantity, CostUnit, RateRow};

use crate::api::requests::video::text_to_video_kling_2_5_turbo::text_to_video_kling_2_5_turbo::TextToVideoKling25TurboRequest;
use crate::api::types::video_types::clip_duration::ClipDuration;
use crate::api::types::video_types::video_resolution::VideoResolution;

/// Official pricing (pricing/base/video): Kling 2.5 Turbo bills per output
/// second — $0.042/s at 720p, $0.07/s at 1080p (no audio option).
pub(crate) const PER_SECOND_720P_MICROS: i64 = 42_000;
pub(crate) const PER_SECOND_1080P_MICROS: i64 = 70_000;

impl CostCalculator for TextToVideoKling25TurboRequest {
  fn estimate_cost(&self) -> CostBreakdown {
    let resolution = self.resolution.unwrap_or(VideoResolution::P720);
    let seconds = self.duration.unwrap_or(ClipDuration::S5).seconds();
    let per_second = match resolution {
      VideoResolution::P720 => PER_SECOND_720P_MICROS,
      VideoResolution::P1080 | VideoResolution::K4 => PER_SECOND_1080P_MICROS,
    };
    CostBreakdown::from_components(
      CostFormula {
        display: "seconds × rate(resolution); billed only on successful generation",
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
        format!("Video output ({}s @ {})", seconds, resolution.as_str()),
        CostUnit::PerVideoSecond,
        per_second,
        CostQuantity::Exact(seconds as f64),
      )],
      false,
    )
  }
}

#[cfg(test)]
mod tests {
  use client_core::{CostCertainty, MicroUsd};

  use super::*;

  fn request(resolution: Option<VideoResolution>, duration: Option<ClipDuration>) -> TextToVideoKling25TurboRequest {
    let mut r = TextToVideoKling25TurboRequest::new("a fox");
    r.resolution = resolution;
    r.duration = duration;
    r
  }

  #[test]
  fn published_rates_at_default_5s() {
    assert_eq!(request(None, None).estimate_cost().amount.micros(), MicroUsd(210_000));
    assert_eq!(
      request(Some(VideoResolution::P1080), None).estimate_cost().amount.micros(),
      MicroUsd(350_000)
    );
  }

  #[test]
  fn same_base_rates_as_kling_2_6_without_audio() {
    // The published table prices 2.5 Turbo and 2.6 (no audio) identically.
    use crate::api::requests::video::text_to_video_kling_2_6::text_to_video_kling_2_6::TextToVideoKling26Request;
    let kling_2_6 = TextToVideoKling26Request::new("a fox").estimate_cost().amount.micros();
    assert_eq!(request(None, None).estimate_cost().amount.micros(), kling_2_6);
  }

  #[test]
  fn exact_certainty_and_10s_doubles() {
    let breakdown = request(None, Some(ClipDuration::S10)).estimate_cost();
    assert_eq!(breakdown.amount.certainty(), CostCertainty::Exact);
    assert_eq!(breakdown.amount.micros(), MicroUsd(420_000));
  }
}
