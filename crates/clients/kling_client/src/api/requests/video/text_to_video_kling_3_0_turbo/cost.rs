use client_core::{CostBreakdown, CostCalculator, CostComponent, CostFormula, CostQuantity, CostUnit, RateRow};

use crate::api::requests::video::text_to_video_kling_3_0_turbo::text_to_video_kling_3_0_turbo::TextToVideoKling30TurboRequest;
use crate::api::types::video_types::video_resolution::VideoResolution;

/// Official pricing (pricing/base/video): Kling 3.0 Turbo bills per output
/// second, native audio included — $0.112/s at 720p, $0.14/s at 1080p.
const PER_SECOND_720P_MICROS: i64 = 112_000;
const PER_SECOND_1080P_MICROS: i64 = 140_000;

pub(crate) const DEFAULT_DURATION_SECONDS: u32 = 5;

fn per_second_micros(resolution: VideoResolution) -> i64 {
  match resolution {
    VideoResolution::P720 => PER_SECOND_720P_MICROS,
    // 4K is rejected at build time; price it as 1080p so an estimate always exists.
    VideoResolution::P1080 | VideoResolution::K4 => PER_SECOND_1080P_MICROS,
  }
}

impl CostCalculator for TextToVideoKling30TurboRequest {
  fn estimate_cost(&self) -> CostBreakdown {
    let resolution = self.resolution.unwrap_or(VideoResolution::P720);
    let seconds = self.duration.unwrap_or(DEFAULT_DURATION_SECONDS);
    let per_second = per_second_micros(resolution);
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

  fn request(resolution: Option<VideoResolution>, duration: Option<u32>) -> TextToVideoKling30TurboRequest {
    let mut r = TextToVideoKling30TurboRequest::new("a fox");
    r.resolution = resolution;
    r.duration = duration;
    r
  }

  mod per_resolution_rates {
    use super::*;

    #[test]
    fn published_rates_for_5s_default() {
      // 5s × $0.112 = $0.56; 5s × $0.14 = $0.70.
      assert_eq!(request(Some(VideoResolution::P720), None).estimate_cost().amount.micros(), MicroUsd(560_000));
      assert_eq!(request(Some(VideoResolution::P1080), None).estimate_cost().amount.micros(), MicroUsd(700_000));
    }

    #[test]
    fn default_resolution_bills_as_720p() {
      assert_eq!(request(None, None).estimate_cost().amount.micros(), MicroUsd(560_000));
    }
  }

  mod duration_scaling {
    use super::*;

    #[test]
    fn cost_scales_linearly_with_seconds() {
      let s3 = request(None, Some(3)).estimate_cost().amount.micros().0;
      let s15 = request(None, Some(15)).estimate_cost().amount.micros().0;
      assert_eq!(s3, 3 * 112_000);
      assert_eq!(s15, 15 * 112_000);
      assert_eq!(s15, 5 * s3);
    }
  }

  #[test]
  fn params_fully_determine_the_bill_so_it_is_exact() {
    let breakdown = request(Some(VideoResolution::P1080), Some(10)).estimate_cost();
    assert_eq!(breakdown.amount.certainty(), CostCertainty::Exact);
    assert_eq!(breakdown.components.len(), 1);
    assert_eq!(breakdown.formula.rates.len(), 2);
  }
}
