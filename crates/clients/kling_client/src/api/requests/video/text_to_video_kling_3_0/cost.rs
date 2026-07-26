use client_core::{CostBreakdown, CostCalculator, CostComponent, CostFormula, CostQuantity, CostUnit, RateRow};

use crate::api::requests::video::text_to_video_kling_3_0::text_to_video_kling_3_0::TextToVideoKling30Request;
use crate::api::types::video_types::native_audio::NativeAudio;
use crate::api::types::video_types::video_resolution::VideoResolution;

/// Official pricing (pricing/base/video), per output second:
///
/// | audio      | 720p   | 1080p  | 4K    |
/// | ---------- | ------ | ------ | ----- |
/// | off        | $0.084 | $0.112 | $0.42 |
/// | native     | $0.126 | $0.168 | $0.42 |
pub(crate) fn per_second_micros(resolution: VideoResolution, native_audio: bool) -> i64 {
  match (resolution, native_audio) {
    (VideoResolution::P720, false) => 84_000,
    (VideoResolution::P1080, false) => 112_000,
    (VideoResolution::P720, true) => 126_000,
    (VideoResolution::P1080, true) => 168_000,
    (VideoResolution::K4, _) => 420_000,
  }
}

const DEFAULT_DURATION_SECONDS: u32 = 5;

pub(crate) fn kling_3_0_rate_rows(native_audio: bool) -> Vec<RateRow> {
  [VideoResolution::P720, VideoResolution::P1080, VideoResolution::K4]
    .iter()
    .map(|resolution| RateRow {
      dimension: "resolution",
      value: resolution.as_str().to_string(),
      unit: CostUnit::PerVideoSecond,
      price_micros: per_second_micros(*resolution, native_audio),
    })
    .collect()
}

impl CostCalculator for TextToVideoKling30Request {
  fn estimate_cost(&self) -> CostBreakdown {
    let resolution = self.resolution.unwrap_or(VideoResolution::P720);
    let native_audio = self.audio == Some(NativeAudio::Native);
    let seconds = self.duration.unwrap_or(DEFAULT_DURATION_SECONDS);
    let per_second = per_second_micros(resolution, native_audio);
    let audio_label = if native_audio { "native audio" } else { "no audio" };
    CostBreakdown::from_components(
      CostFormula {
        display: "seconds × rate(resolution, audio); billed only on successful generation",
        rates: kling_3_0_rate_rows(native_audio),
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
    duration: Option<u32>,
  ) -> TextToVideoKling30Request {
    let mut r = TextToVideoKling30Request::new("a fox");
    r.resolution = resolution;
    r.audio = audio;
    r.duration = duration;
    r
  }

  mod per_rate_card {
    use super::*;

    #[test]
    fn published_rates_no_audio_at_5s() {
      assert_eq!(request(Some(VideoResolution::P720), None, None).estimate_cost().amount.micros(), MicroUsd(420_000));
      assert_eq!(
        request(Some(VideoResolution::P1080), None, None).estimate_cost().amount.micros(),
        MicroUsd(560_000)
      );
      assert_eq!(
        request(Some(VideoResolution::K4), None, None).estimate_cost().amount.micros(),
        MicroUsd(2_100_000)
      );
    }

    #[test]
    fn native_audio_raises_the_sub_4k_rates() {
      assert_eq!(
        request(Some(VideoResolution::P720), Some(NativeAudio::Native), None).estimate_cost().amount.micros(),
        MicroUsd(630_000)
      );
      assert_eq!(
        request(Some(VideoResolution::P1080), Some(NativeAudio::Native), None).estimate_cost().amount.micros(),
        MicroUsd(840_000)
      );
      // 4K is flat regardless of audio.
      assert_eq!(
        request(Some(VideoResolution::K4), Some(NativeAudio::Native), None).estimate_cost().amount.micros(),
        MicroUsd(2_100_000)
      );
    }

    #[test]
    fn audio_off_matches_audio_unset() {
      assert_eq!(
        request(None, Some(NativeAudio::Off), None).estimate_cost().amount.micros(),
        request(None, None, None).estimate_cost().amount.micros()
      );
    }
  }

  mod duration_scaling {
    use super::*;

    #[test]
    fn linear_in_seconds() {
      let s3 = request(None, None, Some(3)).estimate_cost().amount.micros().0;
      let s15 = request(None, None, Some(15)).estimate_cost().amount.micros().0;
      assert_eq!(s3, 3 * 84_000);
      assert_eq!(s15, 15 * 84_000);
    }
  }

  #[test]
  fn exact_certainty_with_full_rate_card() {
    let breakdown = request(Some(VideoResolution::P1080), Some(NativeAudio::Native), Some(10)).estimate_cost();
    assert_eq!(breakdown.amount.certainty(), CostCertainty::Exact);
    assert_eq!(breakdown.formula.rates.len(), 3);
  }
}
