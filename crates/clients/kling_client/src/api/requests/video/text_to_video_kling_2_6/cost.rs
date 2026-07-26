use client_core::{CostBreakdown, CostCalculator, CostComponent, CostFormula, CostQuantity, CostUnit, RateRow};

use crate::api::requests::video::text_to_video_kling_2_6::text_to_video_kling_2_6::TextToVideoKling26Request;
use crate::api::types::video_types::clip_duration::ClipDuration;
use crate::api::types::video_types::native_audio::NativeAudio;
use crate::api::types::video_types::video_resolution::VideoResolution;

/// Official pricing (pricing/base/video), per output second:
///
/// | audio  | 720p   | 1080p  |
/// | ------ | ------ | ------ |
/// | off    | $0.042 | $0.07  |
/// | native | —      | $0.14  |
///
/// (Voice control adds a $0.168/s tier — image-to-video only.)
pub(crate) fn per_second_micros(resolution: VideoResolution, native_audio: bool) -> i64 {
  match (native_audio, resolution) {
    (false, VideoResolution::P720) => 42_000,
    (false, VideoResolution::P1080 | VideoResolution::K4) => 70_000,
    // Native audio is 1080p-only (validated); the rate is flat.
    (true, _) => 140_000,
  }
}

impl CostCalculator for TextToVideoKling26Request {
  fn estimate_cost(&self) -> CostBreakdown {
    let resolution = self.resolution.unwrap_or(VideoResolution::P720);
    let native_audio = self.audio == Some(NativeAudio::Native);
    let seconds = self.duration.unwrap_or(ClipDuration::S5).seconds();
    let per_second = per_second_micros(resolution, native_audio);
    let audio_label = if native_audio { "native audio" } else { "no audio" };
    CostBreakdown::from_components(
      CostFormula {
        display: "seconds × rate(resolution, audio); billed only on successful generation",
        rates: vec![
          RateRow {
            dimension: "resolution",
            value: "720p".to_string(),
            unit: CostUnit::PerVideoSecond,
            price_micros: per_second_micros(VideoResolution::P720, false),
          },
          RateRow {
            dimension: "resolution",
            value: "1080p".to_string(),
            unit: CostUnit::PerVideoSecond,
            price_micros: per_second_micros(VideoResolution::P1080, false),
          },
          RateRow {
            dimension: "audio",
            value: "native (1080p)".to_string(),
            unit: CostUnit::PerVideoSecond,
            price_micros: per_second_micros(VideoResolution::P1080, true),
          },
        ],
      },
      vec![CostComponent::new(
        format!("Video output ({}s @ {}, {})", seconds, resolution.as_str(), audio_label),
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

  fn request(
    resolution: Option<VideoResolution>,
    audio: Option<NativeAudio>,
    duration: Option<ClipDuration>,
  ) -> TextToVideoKling26Request {
    let mut r = TextToVideoKling26Request::new("a fox");
    r.resolution = resolution;
    r.audio = audio;
    r.duration = duration;
    r
  }

  #[test]
  fn published_rates_at_default_5s() {
    assert_eq!(request(None, None, None).estimate_cost().amount.micros(), MicroUsd(210_000));
    assert_eq!(
      request(Some(VideoResolution::P1080), None, None).estimate_cost().amount.micros(),
      MicroUsd(350_000)
    );
    assert_eq!(
      request(Some(VideoResolution::P1080), Some(NativeAudio::Native), None).estimate_cost().amount.micros(),
      MicroUsd(700_000)
    );
  }

  #[test]
  fn ten_second_clip_doubles_the_bill() {
    let s5 = request(None, None, Some(ClipDuration::S5)).estimate_cost().amount.micros().0;
    let s10 = request(None, None, Some(ClipDuration::S10)).estimate_cost().amount.micros().0;
    assert_eq!(s10, 2 * s5);
  }

  #[test]
  fn exact_certainty() {
    assert_eq!(request(None, None, None).estimate_cost().amount.certainty(), CostCertainty::Exact);
  }
}
