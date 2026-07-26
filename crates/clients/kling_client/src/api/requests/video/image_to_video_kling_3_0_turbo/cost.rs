use client_core::{CostBreakdown, CostCalculator, CostComponent, CostFormula, CostQuantity, CostUnit, RateRow};

use crate::api::requests::video::image_to_video_kling_3_0_turbo::image_to_video_kling_3_0_turbo::ImageToVideoKling30TurboRequest;
use crate::api::types::video_types::video_resolution::VideoResolution;

/// Same published rate card as Turbo text-to-video: $0.112/s @720p,
/// $0.14/s @1080p, native audio included.
const PER_SECOND_720P_MICROS: i64 = 112_000;
const PER_SECOND_1080P_MICROS: i64 = 140_000;
const DEFAULT_DURATION_SECONDS: u32 = 5;

impl CostCalculator for ImageToVideoKling30TurboRequest {
  fn estimate_cost(&self) -> CostBreakdown {
    let resolution = self.resolution.unwrap_or(VideoResolution::P720);
    let seconds = self.duration.unwrap_or(DEFAULT_DURATION_SECONDS);
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
      )
      .with_note("native audio included in the per-second rate")],
      false,
    )
  }
}

#[cfg(test)]
mod tests {
  use client_core::{CostCertainty, MicroUsd};

  use super::*;

  fn request(resolution: Option<VideoResolution>, duration: Option<u32>) -> ImageToVideoKling30TurboRequest {
    let mut r = ImageToVideoKling30TurboRequest::new("https://example.com/f.png");
    r.resolution = resolution;
    r.duration = duration;
    r
  }

  #[test]
  fn published_rates_at_default_5s() {
    assert_eq!(request(None, None).estimate_cost().amount.micros(), MicroUsd(560_000));
    assert_eq!(request(Some(VideoResolution::P1080), None).estimate_cost().amount.micros(), MicroUsd(700_000));
  }

  #[test]
  fn matches_text_to_video_turbo_rates() {
    // Turbo bills i2v and t2v identically — keep the two cost tables in lockstep.
    use crate::api::requests::video::text_to_video_kling_3_0_turbo::text_to_video_kling_3_0_turbo::TextToVideoKling30TurboRequest;
    let mut t2v = TextToVideoKling30TurboRequest::new("a fox");
    t2v.duration = Some(7);
    let i2v = request(None, Some(7));
    assert_eq!(t2v.estimate_cost().amount.micros(), i2v.estimate_cost().amount.micros());
  }

  #[test]
  fn exact_certainty_and_linear_scaling() {
    let breakdown = request(None, Some(15)).estimate_cost();
    assert_eq!(breakdown.amount.certainty(), CostCertainty::Exact);
    assert_eq!(breakdown.amount.micros(), MicroUsd(15 * 112_000));
  }
}
