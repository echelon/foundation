use client_core::{CostBreakdown, CostCalculator, CostComponent, CostFormula, CostQuantity, CostUnit};

use crate::api::requests::video::image_to_video_kling_3_0::image_to_video_kling_3_0::ImageToVideoKling30Request;
use crate::api::requests::video::text_to_video_kling_3_0::cost::{kling_3_0_rate_rows, per_second_micros};
use crate::api::types::video_types::native_audio::NativeAudio;
use crate::api::types::video_types::video_resolution::VideoResolution;

const DEFAULT_DURATION_SECONDS: u32 = 5;

/// Same rate card as Kling 3.0 text-to-video (the pricing table is
/// per-model, not per-mode).
impl CostCalculator for ImageToVideoKling30Request {
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

  fn request(resolution: Option<VideoResolution>, audio: Option<NativeAudio>) -> ImageToVideoKling30Request {
    let mut r = ImageToVideoKling30Request::new("https://example.com/f.png");
    r.resolution = resolution;
    r.audio = audio;
    r
  }

  #[test]
  fn published_rates_at_default_5s() {
    assert_eq!(request(None, None).estimate_cost().amount.micros(), MicroUsd(420_000));
    assert_eq!(
      request(Some(VideoResolution::K4), None).estimate_cost().amount.micros(),
      MicroUsd(2_100_000)
    );
    assert_eq!(
      request(Some(VideoResolution::P1080), Some(NativeAudio::Native)).estimate_cost().amount.micros(),
      MicroUsd(840_000)
    );
  }

  #[test]
  fn matches_text_to_video_3_0_rates() {
    use crate::api::requests::video::text_to_video_kling_3_0::text_to_video_kling_3_0::TextToVideoKling30Request;
    let mut t2v = TextToVideoKling30Request::new("a fox");
    t2v.audio = Some(NativeAudio::Native);
    let i2v = request(None, Some(NativeAudio::Native));
    assert_eq!(t2v.estimate_cost().amount.micros(), i2v.estimate_cost().amount.micros());
  }

  #[test]
  fn elements_do_not_change_the_bill() {
    use crate::api::types::video_types::element_ref::ElementRef;
    let plain = request(None, None).estimate_cost().amount.micros();
    let mut with_elements = request(None, None);
    with_elements.elements = vec![ElementRef { element_id: "1".to_string(), id: "a".to_string() }];
    assert_eq!(with_elements.estimate_cost().amount.micros(), plain);
  }

  #[test]
  fn exact_certainty() {
    assert_eq!(request(None, None).estimate_cost().amount.certainty(), CostCertainty::Exact);
  }
}
