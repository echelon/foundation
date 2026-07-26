use client_core::{CostBreakdown, CostCalculator, CostComponent, CostFormula, CostQuantity, CostUnit, RateRow};

use crate::api::requests::audio::video_to_audio::video_to_audio::VideoToAudioRequest;

/// Official pricing (pricing/base/video): Video to Audio is a flat
/// 0.25 units — $0.035 per call, same as text-to-audio.
const PER_CALL_MICROS: i64 = 35_000;

impl CostCalculator for VideoToAudioRequest {
  fn estimate_cost(&self) -> CostBreakdown {
    CostBreakdown::from_components(
      CostFormula {
        display: "flat per generation call",
        rates: vec![RateRow {
          dimension: "—",
          value: "per call".to_string(),
          unit: CostUnit::PerRequest,
          price_micros: PER_CALL_MICROS,
        }],
      },
      vec![CostComponent::new("Video-to-audio generation", CostUnit::PerRequest, PER_CALL_MICROS, CostQuantity::Exact(1.0))
        .with_note("source video length (3–20s) does not affect the flat rate")],
      false,
    )
  }
}

#[cfg(test)]
mod tests {
  use client_core::{CostCertainty, MicroUsd};

  use super::*;
  use crate::api::requests::audio::video_to_audio::video_to_audio::VideoToAudioSource;

  #[test]
  fn flat_rate_matches_text_to_audio() {
    use crate::api::requests::audio::text_to_audio::text_to_audio::TextToAudioRequest;
    let v2a = VideoToAudioRequest::new(VideoToAudioSource::VideoId("1".to_string())).estimate_cost();
    let t2a = TextToAudioRequest::new("wind", 5.0).estimate_cost();
    assert_eq!(v2a.amount.micros(), MicroUsd(35_000));
    assert_eq!(v2a.amount.micros(), t2a.amount.micros());
    assert_eq!(v2a.amount.certainty(), CostCertainty::Exact);
  }
}
