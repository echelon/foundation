use client_core::{BetterCostEstimator, CostBreakdown, CostComponent, CostFormula, CostQuantity, CostUnit, RateRow};

use crate::api::requests::video::create_task::request_types::CreateTaskRequest;
use crate::api::types::video_types::seedance_model::SeedanceModel;

// Kinovi bills Seedance video per output SECOND. Rates below are in micro-USD
// per video-second:
//
//   seedance-20        120_000  ($0.12/sec)
//   seedance2-fast      60_000  ($0.06/sec)
//   seedance2.0-mini    30_000  ($0.03/sec)
//   Custom(_)          120_000  (unknown model → assume the flagship rate)
//
// NOTE: pricing is duration-based only for now. Higher resolutions (notably 4k)
// may bill an upscale surcharge upstream, but until Kinovi documents a stable
// resolution multiplier we keep the estimate a pure rate × duration so the
// pre-flight number is predictable. Revisit if/when 4k upscale billing is confirmed.

/// Micro-USD charged per video-second for a given model.
pub(crate) fn micros_per_second(model: &SeedanceModel) -> i64 {
  match model {
    SeedanceModel::Seedance20     => 120_000,
    SeedanceModel::Seedance2Fast  => 60_000,
    SeedanceModel::Seedance20Mini => 30_000,
    // Unknown/future model: conservatively assume the most expensive tier.
    SeedanceModel::Custom(_)      => 120_000,
  }
}

impl BetterCostEstimator for CreateTaskRequest {
  /// Estimated charge = per-second rate × clip duration. ESTIMATED (not exact)
  /// because higher resolutions may bill an upstream upscale surcharge Kinovi
  /// hasn't documented — see the module note.
  fn estimate_cost(&self) -> CostBreakdown {
    let rate = micros_per_second(&self.model);
    CostBreakdown::from_components(
      CostFormula {
        display: "seconds × rate(model); possible undocumented 4k upscale surcharge",
        rates: [
          (SeedanceModel::Seedance20, "seedance-20"),
          (SeedanceModel::Seedance2Fast, "seedance2-fast"),
          (SeedanceModel::Seedance20Mini, "seedance2.0-mini"),
        ]
        .iter()
        .map(|(m, label)| RateRow {
          dimension: "model",
          value: label.to_string(),
          unit: CostUnit::PerVideoSecond,
          price_micros: micros_per_second(m),
        })
        .collect(),
      },
      vec![
        CostComponent::new(
          format!("Video output ({}s @ {})", self.inputs.duration_seconds, self.inputs.resolution.as_str()),
          CostUnit::PerVideoSecond,
          rate,
          CostQuantity::Exact(self.inputs.duration_seconds as f64),
        )
        .with_note("possible undocumented resolution surcharge at 4k"),
      ],
      true,
    )
  }
}

#[cfg(test)]
mod tests {
  use client_core::MicroUsd;

  use super::*;
  use crate::api::requests::video::create_task::request_types::VideoInputs;
  use crate::api::types::video_types::video_aspect_ratio::VideoAspectRatio;
  use crate::api::types::video_types::video_resolution::VideoResolution;

  fn request(model: SeedanceModel, duration_seconds: u32, resolution: VideoResolution) -> CreateTaskRequest {
    CreateTaskRequest {
      model,
      inputs: VideoInputs {
        prompt: "test".to_string(),
        duration_seconds,
        resolution,
        aspect_ratio: Some(VideoAspectRatio::Landscape16x9),
        image_urls: vec![],
      },
      callback_url: None,
    }
  }

  // ── Per-model rate (at 1 second the cost equals the rate) ──

  mod per_model_rates {
    use super::*;

    #[test]
    fn seedance20_is_120k_micros_per_second() {
      let req = request(SeedanceModel::Seedance20, 1, VideoResolution::P720);
      assert_eq!(req.estimate_cost().amount.micros(), MicroUsd(120_000));
    }

    #[test]
    fn fast_is_60k_micros_per_second() {
      let req = request(SeedanceModel::Seedance2Fast, 1, VideoResolution::P720);
      assert_eq!(req.estimate_cost().amount.micros(), MicroUsd(60_000));
    }

    #[test]
    fn mini_is_30k_micros_per_second() {
      let req = request(SeedanceModel::Seedance20Mini, 1, VideoResolution::P720);
      assert_eq!(req.estimate_cost().amount.micros(), MicroUsd(30_000));
    }

    #[test]
    fn custom_falls_back_to_flagship_rate() {
      let req = request(SeedanceModel::Custom("seedance-30".to_string()), 1, VideoResolution::P720);
      assert_eq!(req.estimate_cost().amount.micros(), MicroUsd(120_000));
    }
  }

  // ── Duration scaling ──

  mod duration_scaling {
    use super::*;

    #[test]
    fn five_second_seedance20_is_600k_micros() {
      // $0.12/sec × 5s = $0.60 = 600_000 micros.
      let req = request(SeedanceModel::Seedance20, 5, VideoResolution::P1080);
      assert_eq!(req.estimate_cost().amount.micros(), MicroUsd(600_000));
    }

    #[test]
    fn cost_scales_linearly_with_duration() {
      for model in [
        SeedanceModel::Seedance20,
        SeedanceModel::Seedance2Fast,
        SeedanceModel::Seedance20Mini,
      ] {
        let rate = micros_per_second(&model);
        for duration in 4u32..=15 {
          let req = request(model.clone(), duration, VideoResolution::P720);
          assert_eq!(
            req.estimate_cost().amount.micros(),
            MicroUsd(rate * duration as i64),
            "model={} duration={duration}",
            model.as_str()
          );
        }
      }
    }

    #[test]
    fn zero_duration_is_zero_cost() {
      let req = request(SeedanceModel::Seedance20, 0, VideoResolution::P720);
      assert_eq!(req.estimate_cost().amount.micros(), MicroUsd(0));
    }
  }

  // ── Resolution independence (duration-based billing for now) ──

  mod resolution_independence {
    use super::*;

    #[test]
    fn cost_is_independent_of_resolution() {
      for resolution in [
        VideoResolution::P480,
        VideoResolution::P720,
        VideoResolution::P1080,
        VideoResolution::K4,
      ] {
        let req = request(SeedanceModel::Seedance20, 6, resolution);
        // Same as 720p at the same duration — resolution does not (yet) move the bill.
        assert_eq!(req.estimate_cost().amount.micros(), MicroUsd(120_000 * 6), "resolution={resolution:?}");
      }
    }
  }

  // ── Independence from non-billing fields ──

  mod field_independence {
    use super::*;

    #[test]
    fn cost_is_independent_of_aspect_ratio() {
      let mut req = request(SeedanceModel::Seedance2Fast, 5, VideoResolution::P720);
      let base = req.estimate_cost().amount.micros();
      for ar in [
        VideoAspectRatio::Landscape16x9,
        VideoAspectRatio::Portrait9x16,
        VideoAspectRatio::Square1x1,
        VideoAspectRatio::Cinematic21x9,
      ] {
        req.inputs.aspect_ratio = Some(ar);
        assert_eq!(req.estimate_cost().amount.micros(), base, "{ar:?}");
      }
      req.inputs.aspect_ratio = None;
      assert_eq!(req.estimate_cost().amount.micros(), base);
    }

    #[test]
    fn cost_is_independent_of_image_urls_and_callback() {
      let mut req = request(SeedanceModel::Seedance20Mini, 8, VideoResolution::P720);
      let base = req.estimate_cost().amount.micros();
      req.inputs.image_urls = vec!["https://example.com/frame.png".to_string()];
      req.callback_url = Some("https://example.com/webhook".to_string());
      assert_eq!(req.estimate_cost().amount.micros(), base);
    }
  }

  // ── Monotonicity sanity ──

  mod monotonicity {
    use super::*;

    #[test]
    fn flagship_at_least_as_expensive_as_fast_at_least_as_expensive_as_mini() {
      let mini = request(SeedanceModel::Seedance20Mini, 5, VideoResolution::P720).estimate_cost().amount.micros();
      let fast = request(SeedanceModel::Seedance2Fast, 5, VideoResolution::P720).estimate_cost().amount.micros();
      let flagship = request(SeedanceModel::Seedance20, 5, VideoResolution::P720).estimate_cost().amount.micros();
      assert!(mini <= fast, "mini ({mini:?}) should be <= fast ({fast:?})");
      assert!(fast <= flagship, "fast ({fast:?}) should be <= flagship ({flagship:?})");
    }
  }
}
