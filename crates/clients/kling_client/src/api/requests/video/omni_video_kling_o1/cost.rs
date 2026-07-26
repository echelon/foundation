use client_core::{CostBreakdown, CostCalculator, CostComponent, CostFormula, CostQuantity, CostUnit, RateRow};

use crate::api::requests::video::omni_video_kling_o1::omni_video_kling_o1::OmniVideoKlingO1Request;
use crate::api::types::video_types::video_resolution::VideoResolution;

/// Official pricing (pricing/base/video), per output second:
///
/// | inputs         | 720p   | 1080p  |
/// | -------------- | ------ | ------ |
/// | no video input | $0.084 | $0.112 |
/// | video input    | $0.126 | $0.168 |
fn per_second_micros(resolution: VideoResolution, has_video_input: bool) -> i64 {
  match (has_video_input, resolution) {
    (false, VideoResolution::P720) => 84_000,
    (true, VideoResolution::P720) => 126_000,
    // 4K is rejected at build time; price any residual as 1080p.
    (false, VideoResolution::P1080 | VideoResolution::K4) => 112_000,
    (true, VideoResolution::P1080 | VideoResolution::K4) => 168_000,
  }
}

const DEFAULT_DURATION_SECONDS: u32 = 5;

impl CostCalculator for OmniVideoKlingO1Request {
  fn estimate_cost(&self) -> CostBreakdown {
    let resolution = self.resolution.unwrap_or(VideoResolution::P720);
    let has_video_input = self.reference_video.is_some();
    let seconds = self.duration.unwrap_or(DEFAULT_DURATION_SECONDS);
    let per_second = per_second_micros(resolution, has_video_input);
    let tier_label = if has_video_input { "video input" } else { "no video input" };
    CostBreakdown::from_components(
      CostFormula {
        display: "seconds × rate(resolution, video-input); billed only on successful generation",
        rates: [VideoResolution::P720, VideoResolution::P1080]
          .iter()
          .map(|r| RateRow {
            dimension: "resolution",
            value: r.as_str().to_string(),
            unit: CostUnit::PerVideoSecond,
            price_micros: per_second_micros(*r, has_video_input),
          })
          .collect(),
      },
      vec![CostComponent::new(
        format!("Video output ({}s @ {}, {})", seconds, resolution.as_str(), tier_label),
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
  use crate::api::requests::video::omni_video_kling_3_0_omni::omni_video_kling_3_0_omni::OmniReferenceVideo;
  use crate::api::types::video_types::refer_video_input::ReferVideoInput;

  fn request(resolution: Option<VideoResolution>, with_video: bool) -> OmniVideoKlingO1Request {
    let mut r = OmniVideoKlingO1Request::new("a fox");
    r.resolution = resolution;
    if with_video {
      r.reference_video =
        Some(OmniReferenceVideo::Feature(ReferVideoInput { url: "https://example.com/s.mp4".to_string(), id: None }));
    }
    r
  }

  #[test]
  fn published_rates_at_default_5s() {
    assert_eq!(request(None, false).estimate_cost().amount.micros(), MicroUsd(420_000));
    assert_eq!(
      request(Some(VideoResolution::P1080), false).estimate_cost().amount.micros(),
      MicroUsd(560_000)
    );
    assert_eq!(request(None, true).estimate_cost().amount.micros(), MicroUsd(630_000));
    assert_eq!(
      request(Some(VideoResolution::P1080), true).estimate_cost().amount.micros(),
      MicroUsd(840_000)
    );
  }

  #[test]
  fn o1_no_video_tier_matches_kling_3_0_base_rate() {
    // The pricing table lists O1 (no video input) at the same $0.084/$0.112
    // per-second rates as Kling 3.0 without audio — assert the two cost
    // tables agree so a one-sided edit gets caught.
    use crate::api::requests::video::text_to_video_kling_3_0::text_to_video_kling_3_0::TextToVideoKling30Request;
    let kling_3_0 = TextToVideoKling30Request::new("a fox").estimate_cost().amount.micros();
    assert_eq!(request(None, false).estimate_cost().amount.micros(), kling_3_0);
  }

  #[test]
  fn exact_certainty_and_duration_scaling() {
    let mut r = request(None, false);
    r.duration = Some(10);
    let breakdown = r.estimate_cost();
    assert_eq!(breakdown.amount.certainty(), CostCertainty::Exact);
    assert_eq!(breakdown.amount.micros(), MicroUsd(840_000));
    assert_eq!(breakdown.formula.rates.len(), 2);
  }
}
