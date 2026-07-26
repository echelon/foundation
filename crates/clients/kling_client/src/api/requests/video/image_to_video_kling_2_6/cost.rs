use client_core::{CostBreakdown, CostCalculator, CostComponent, CostFormula, CostQuantity, CostUnit, RateRow};

use crate::api::requests::video::image_to_video_kling_2_6::image_to_video_kling_2_6::ImageToVideoKling26Request;
use crate::api::types::video_types::clip_duration::ClipDuration;
use crate::api::types::video_types::native_audio::NativeAudio;
use crate::api::types::video_types::video_resolution::VideoResolution;

/// Official pricing (pricing/base/video), per output second:
///
/// | tier                        | 720p   | 1080p  |
/// | --------------------------- | ------ | ------ |
/// | no audio                    | $0.042 | $0.07  |
/// | native audio, no voices     | —      | $0.14  |
/// | native audio, voice control | —      | $0.168 |
fn per_second_micros(resolution: VideoResolution, native_audio: bool, voice_control: bool) -> i64 {
  match (native_audio, voice_control, resolution) {
    (false, _, VideoResolution::P720) => 42_000,
    (false, _, VideoResolution::P1080 | VideoResolution::K4) => 70_000,
    (true, false, _) => 140_000,
    (true, true, _) => 168_000,
  }
}

impl CostCalculator for ImageToVideoKling26Request {
  fn estimate_cost(&self) -> CostBreakdown {
    let resolution = self.resolution.unwrap_or(VideoResolution::P720);
    let native_audio = self.audio == Some(NativeAudio::Native);
    let voice_control = !self.voices.is_empty();
    let seconds = self.duration.unwrap_or(ClipDuration::S5).seconds();
    let per_second = per_second_micros(resolution, native_audio, voice_control);
    let tier_label = match (native_audio, voice_control) {
      (false, _) => "no audio",
      (true, false) => "native audio",
      (true, true) => "native audio + voice control",
    };
    CostBreakdown::from_components(
      CostFormula {
        display: "seconds × rate(resolution, audio, voice-control); billed only on successful generation",
        rates: vec![
          RateRow {
            dimension: "resolution",
            value: "720p".to_string(),
            unit: CostUnit::PerVideoSecond,
            price_micros: per_second_micros(VideoResolution::P720, false, false),
          },
          RateRow {
            dimension: "resolution",
            value: "1080p".to_string(),
            unit: CostUnit::PerVideoSecond,
            price_micros: per_second_micros(VideoResolution::P1080, false, false),
          },
          RateRow {
            dimension: "audio",
            value: "native (1080p)".to_string(),
            unit: CostUnit::PerVideoSecond,
            price_micros: per_second_micros(VideoResolution::P1080, true, false),
          },
          RateRow {
            dimension: "audio",
            value: "native + voices (1080p)".to_string(),
            unit: CostUnit::PerVideoSecond,
            price_micros: per_second_micros(VideoResolution::P1080, true, true),
          },
        ],
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
  use crate::api::types::video_types::voice_ref::VoiceRef;

  fn request() -> ImageToVideoKling26Request {
    ImageToVideoKling26Request::new("https://example.com/f.png")
  }

  #[test]
  fn published_rates_at_default_5s() {
    assert_eq!(request().estimate_cost().amount.micros(), MicroUsd(210_000));

    let mut hd = request();
    hd.resolution = Some(VideoResolution::P1080);
    assert_eq!(hd.estimate_cost().amount.micros(), MicroUsd(350_000));

    let mut native = request();
    native.audio = Some(NativeAudio::Native);
    native.resolution = Some(VideoResolution::P1080);
    assert_eq!(native.estimate_cost().amount.micros(), MicroUsd(700_000));

    let mut voiced = native.clone();
    voiced.voices = vec![VoiceRef { voice_id: "v".to_string(), id: "1".to_string() }];
    assert_eq!(voiced.estimate_cost().amount.micros(), MicroUsd(840_000));
  }

  #[test]
  fn voice_control_tier_beats_plain_native_audio() {
    let mut native = request();
    native.audio = Some(NativeAudio::Native);
    native.resolution = Some(VideoResolution::P1080);
    let mut voiced = native.clone();
    voiced.voices = vec![VoiceRef { voice_id: "v".to_string(), id: "1".to_string() }];
    assert!(voiced.estimate_cost().amount.micros().0 > native.estimate_cost().amount.micros().0);
  }

  #[test]
  fn matches_text_to_video_2_6_for_shared_tiers() {
    use crate::api::requests::video::text_to_video_kling_2_6::text_to_video_kling_2_6::TextToVideoKling26Request;
    let t2v = TextToVideoKling26Request::new("a fox").estimate_cost().amount.micros();
    assert_eq!(request().estimate_cost().amount.micros(), t2v);
  }

  #[test]
  fn exact_certainty_and_duration_scaling() {
    let mut r = request();
    r.duration = Some(ClipDuration::S10);
    let breakdown = r.estimate_cost();
    assert_eq!(breakdown.amount.certainty(), CostCertainty::Exact);
    assert_eq!(breakdown.amount.micros(), MicroUsd(420_000));
    assert_eq!(breakdown.formula.rates.len(), 4);
  }
}
