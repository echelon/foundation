use client_core::{CostBreakdown, CostCalculator, CostComponent, CostFormula, CostQuantity, CostUnit, RateRow};

use crate::api::requests::audio::text_to_audio::text_to_audio::TextToAudioRequest;

/// Official pricing (pricing/base/video): Text to Audio is a flat
/// 0.25 units — $0.035 per call, regardless of duration.
pub(crate) const PER_CALL_MICROS: i64 = 35_000;

impl CostCalculator for TextToAudioRequest {
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
      vec![CostComponent::new("Text-to-audio generation", CostUnit::PerRequest, PER_CALL_MICROS, CostQuantity::Exact(1.0))
        .with_note("duration (3–10s) does not affect the flat rate")],
      false,
    )
  }
}

#[cfg(test)]
mod tests {
  use client_core::{CostCertainty, MicroUsd};

  use super::*;

  #[test]
  fn flat_rate_independent_of_duration() {
    let short = TextToAudioRequest::new("wind", 3.0).estimate_cost();
    let long = TextToAudioRequest::new("wind", 10.0).estimate_cost();
    assert_eq!(short.amount.micros(), MicroUsd(35_000));
    assert_eq!(long.amount.micros(), MicroUsd(35_000));
    assert_eq!(short.amount.certainty(), CostCertainty::Exact);
  }
}
