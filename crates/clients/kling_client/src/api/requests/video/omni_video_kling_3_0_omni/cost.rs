use client_core::{CostBreakdown, CostCalculator, CostComponent, CostFormula, CostQuantity, CostUnit, RateRow};

use crate::api::requests::video::omni_video_kling_3_0_omni::omni_video_kling_3_0_omni::OmniVideoKling30OmniRequest;
use crate::api::types::video_types::omni_audio::OmniAudio;
use crate::api::types::video_types::video_resolution::VideoResolution;

/// Official pricing (pricing/base/video), per output second:
///
/// | inputs                       | 720p   | 1080p  | 4K    |
/// | ---------------------------- | ------ | ------ | ----- |
/// | no video input, no audio     | $0.084 | $0.112 | $0.42 |
/// | no video input, native audio | $0.112 | $0.14  | $0.42 |
/// | video input (audio ≠ native) | $0.126 | $0.168 | $0.42 |
///
/// (video input + native audio is not a legal request — validation rejects it.)
fn per_second_micros(resolution: VideoResolution, has_video_input: bool, native_audio: bool) -> i64 {
  if resolution == VideoResolution::K4 {
    return 420_000;
  }
  match (has_video_input, native_audio, resolution) {
    (false, false, VideoResolution::P720) => 84_000,
    (false, false, VideoResolution::P1080) => 112_000,
    (false, true, VideoResolution::P720) => 112_000,
    (false, true, VideoResolution::P1080) => 140_000,
    (true, _, VideoResolution::P720) => 126_000,
    (true, _, VideoResolution::P1080) => 168_000,
    (_, _, VideoResolution::K4) => 420_000,
  }
}

const DEFAULT_DURATION_SECONDS: u32 = 5;

impl CostCalculator for OmniVideoKling30OmniRequest {
  fn estimate_cost(&self) -> CostBreakdown {
    let resolution = self.resolution.unwrap_or(VideoResolution::P720);
    let has_video_input = self.reference_video.is_some();
    let native_audio = self.audio == Some(OmniAudio::Native);
    let seconds = self.duration.unwrap_or(DEFAULT_DURATION_SECONDS);
    let per_second = per_second_micros(resolution, has_video_input, native_audio);
    let tier_label = match (has_video_input, native_audio) {
      (true, _) => "video input",
      (false, true) => "native audio",
      (false, false) => "no video input, no audio",
    };
    CostBreakdown::from_components(
      CostFormula {
        display: "seconds × rate(resolution, video-input, audio); billed only on successful generation",
        rates: [VideoResolution::P720, VideoResolution::P1080, VideoResolution::K4]
          .iter()
          .map(|r| RateRow {
            dimension: "resolution",
            value: r.as_str().to_string(),
            unit: CostUnit::PerVideoSecond,
            price_micros: per_second_micros(*r, has_video_input, native_audio),
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

  fn base_request() -> OmniVideoKling30OmniRequest {
    OmniVideoKling30OmniRequest::new("a fox")
  }

  fn with_base_video() -> OmniVideoKling30OmniRequest {
    let mut r = base_request();
    r.reference_video =
      Some(OmniReferenceVideo::Base(ReferVideoInput { url: "https://example.com/s.mp4".to_string(), id: None }));
    r
  }

  mod per_tier_rates {
    use super::*;

    #[test]
    fn no_video_no_audio_at_5s() {
      assert_eq!(base_request().estimate_cost().amount.micros(), MicroUsd(420_000));
    }

    #[test]
    fn native_audio_tier_at_5s() {
      let mut r = base_request();
      r.audio = Some(OmniAudio::Native);
      assert_eq!(r.estimate_cost().amount.micros(), MicroUsd(560_000));
    }

    #[test]
    fn video_input_tier_at_5s() {
      assert_eq!(with_base_video().estimate_cost().amount.micros(), MicroUsd(630_000));
      let mut r1080 = with_base_video();
      r1080.resolution = Some(VideoResolution::P1080);
      assert_eq!(r1080.estimate_cost().amount.micros(), MicroUsd(840_000));
    }

    #[test]
    fn four_k_is_flat_across_tiers() {
      let mut plain = base_request();
      plain.resolution = Some(VideoResolution::K4);
      let mut with_video = with_base_video();
      with_video.resolution = Some(VideoResolution::K4);
      assert_eq!(plain.estimate_cost().amount.micros(), MicroUsd(2_100_000));
      assert_eq!(with_video.estimate_cost().amount.micros(), MicroUsd(2_100_000));
    }
  }

  #[test]
  fn feature_and_base_video_bill_the_same_tier() {
    let mut feature = base_request();
    feature.reference_video =
      Some(OmniReferenceVideo::Feature(ReferVideoInput { url: "https://example.com/s.mp4".to_string(), id: None }));
    assert_eq!(
      feature.estimate_cost().amount.micros(),
      with_base_video().estimate_cost().amount.micros()
    );
  }

  #[test]
  fn exact_certainty_and_duration_scaling() {
    let mut r = base_request();
    r.duration = Some(15);
    let breakdown = r.estimate_cost();
    assert_eq!(breakdown.amount.certainty(), CostCertainty::Exact);
    assert_eq!(breakdown.amount.micros(), MicroUsd(15 * 84_000));
  }
}
