use client_core::{CostBreakdown, CostCalculator, CostComponent, CostFormula, CostQuantity, CostUnit, RateRow};

use crate::api::requests::video::image_to_video_kling_2_5_turbo::image_to_video_kling_2_5_turbo::ImageToVideoKling25TurboRequest;
use crate::api::requests::video::text_to_video_kling_2_5_turbo::cost::{
  PER_SECOND_1080P_MICROS, PER_SECOND_720P_MICROS,
};
use crate::api::types::video_types::clip_duration::ClipDuration;
use crate::api::types::video_types::video_resolution::VideoResolution;

/// Same rate card as 2.5 Turbo text-to-video: $0.042/s @720p, $0.07/s @1080p.
impl CostCalculator for ImageToVideoKling25TurboRequest {
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

  fn request(resolution: Option<VideoResolution>) -> ImageToVideoKling25TurboRequest {
    let mut r = ImageToVideoKling25TurboRequest::new("https://example.com/f.png");
    r.resolution = resolution;
    r
  }

  #[test]
  fn published_rates_at_default_5s() {
    assert_eq!(request(None).estimate_cost().amount.micros(), MicroUsd(210_000));
    assert_eq!(
      request(Some(VideoResolution::P1080)).estimate_cost().amount.micros(),
      MicroUsd(350_000)
    );
  }

  #[test]
  fn matches_text_to_video_2_5_turbo_rates() {
    use crate::api::requests::video::text_to_video_kling_2_5_turbo::text_to_video_kling_2_5_turbo::TextToVideoKling25TurboRequest;
    let t2v = TextToVideoKling25TurboRequest::new("a fox").estimate_cost().amount.micros();
    assert_eq!(request(None).estimate_cost().amount.micros(), t2v);
  }

  #[test]
  fn exact_certainty() {
    assert_eq!(request(None).estimate_cost().amount.certainty(), CostCertainty::Exact);
  }
}
