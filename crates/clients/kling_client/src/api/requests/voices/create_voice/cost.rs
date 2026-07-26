use client_core::{CostBreakdown, CostCalculator, CostComponent, CostFormula, CostQuantity, CostUnit, RateRow};

use crate::api::requests::voices::create_voice::create_voice::CreateVoiceRequest;

/// Official pricing (pricing/base/video): Custom Voice is a flat
/// 0.05 units — $0.007 per call.
const PER_CALL_MICROS: i64 = 7_000;

impl CostCalculator for CreateVoiceRequest {
  fn estimate_cost(&self) -> CostBreakdown {
    CostBreakdown::from_components(
      CostFormula {
        display: "flat per creation call",
        rates: vec![RateRow {
          dimension: "—",
          value: "per voice".to_string(),
          unit: CostUnit::PerRequest,
          price_micros: PER_CALL_MICROS,
        }],
      },
      vec![CostComponent::new("Custom voice creation", CostUnit::PerRequest, PER_CALL_MICROS, CostQuantity::Exact(1.0))],
      false,
    )
  }
}

#[cfg(test)]
mod tests {
  use client_core::{CostCertainty, MicroUsd};

  use super::*;
  use crate::api::requests::voices::create_voice::create_voice::VoiceSource;

  #[test]
  fn flat_published_rate_regardless_of_source() {
    let from_url = CreateVoiceRequest {
      voice_name: "n".to_string(),
      source: VoiceSource::Url("https://example.com/s.mp3".to_string()),
      callback_url: None,
      external_task_id: None,
    };
    let from_video = CreateVoiceRequest {
      voice_name: "n".to_string(),
      source: VoiceSource::VideoId("1".to_string()),
      callback_url: None,
      external_task_id: None,
    };
    assert_eq!(from_url.estimate_cost().amount.micros(), MicroUsd(7_000));
    assert_eq!(from_video.estimate_cost().amount.micros(), MicroUsd(7_000));
    assert_eq!(from_url.estimate_cost().amount.certainty(), CostCertainty::Exact);
  }
}
