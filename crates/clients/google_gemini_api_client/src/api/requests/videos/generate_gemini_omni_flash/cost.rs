use client_core::{CostCalculator, CostCertainty, MicroUsd};

use crate::api::requests::videos::generate_gemini_omni_flash::generate_gemini_omni_flash::GeminiOmniFlashRequest;

// Gemini Omni Flash (gemini-omni-flash-preview): TOKEN-billed — $17.50 / 1M
// video output tokens ≈ $0.10 per second of 720p video (plus $9.00 / 1M text
// output tokens, negligible here). Output length is prompt-driven with no
// duration parameter, so the price can only be ESTIMATED up front; the
// provider's reported usage is authoritative after the fact.

const MICROS_PER_ESTIMATED_SECOND: i64 = 100_000;
const DEFAULT_ESTIMATED_SECONDS: i64 = 8;

impl CostCalculator for GeminiOmniFlashRequest {
  fn cost_micros(&self) -> MicroUsd {
    let seconds = self.expected_output_seconds.unwrap_or(DEFAULT_ESTIMATED_SECONDS);
    MicroUsd(MICROS_PER_ESTIMATED_SECOND * seconds)
  }

  /// Token-billed with prompt-driven output length — always an estimate.
  fn cost_certainty(&self) -> CostCertainty {
    CostCertainty::Estimated
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn req(expected: Option<i64>) -> GeminiOmniFlashRequest {
    GeminiOmniFlashRequest {
      prompt: "p".into(),
      images: vec![],
      video_file_uri: None,
      task: None,
      aspect_ratio: None,
      delivery: None,
      previous_interaction_id: None,
      store: None,
      expected_output_seconds: expected,
    }
  }

  #[test]
  fn estimates_by_expected_seconds() {
    assert_eq!(req(None).cost_micros(), MicroUsd(800_000)); // 8 s default ≈ $0.80
    assert_eq!(req(Some(4)).cost_micros(), MicroUsd(400_000));
    assert_eq!(req(None).cost_certainty(), CostCertainty::Estimated);
  }
}


use client_core::{BetterCostEstimator, CostBreakdown, CostComponent, CostFormula, CostQuantity, CostUnit, RateRow};

impl BetterCostEstimator for GeminiOmniFlashRequest {
  fn estimate_cost(&self) -> CostBreakdown {
    let seconds = self.expected_output_seconds.unwrap_or(DEFAULT_ESTIMATED_SECONDS);
    CostBreakdown::from_components(
      CostFormula {
        display: "output video tokens × $17.50/1M (≈ $0.10 per 720p second; length is prompt-driven)",
        rates: vec![
          RateRow {
            dimension: "output",
            value: "video (≈ per 720p second)".to_string(),
            unit: CostUnit::PerVideoSecond,
            price_micros: MICROS_PER_ESTIMATED_SECOND,
          },
          RateRow {
            dimension: "output",
            value: "video tokens".to_string(),
            unit: CostUnit::PerMillionOutputTokens,
            price_micros: 17_500_000,
          },
          RateRow {
            dimension: "output",
            value: "text tokens".to_string(),
            unit: CostUnit::PerMillionOutputTokens,
            price_micros: 9_000_000,
          },
        ],
      },
      vec![
        CostComponent::new(
          format!("Video output (~{seconds}s @ ~720p)"),
          CostUnit::PerVideoSecond,
          MICROS_PER_ESTIMATED_SECOND,
          CostQuantity::Estimated(seconds as f64),
        )
        .with_note("token-billed; output length is prompt-driven — provider usage is authoritative"),
      ],
      true,
    )
  }
}

#[cfg(test)]
mod better_tests {
  use super::*;
  use client_core::CostCertainty;

  #[test]
  fn matches_legacy_and_is_estimated() {
    let request = GeminiOmniFlashRequest {
      prompt: "p".into(), images: vec![], video_file_uri: None, task: None, aspect_ratio: None,
      delivery: None, previous_interaction_id: None, store: None, expected_output_seconds: Some(4),
    };
    let breakdown = request.estimate_cost();
    assert_eq!(breakdown.amount.micros(), request.cost_micros());
    assert_eq!(breakdown.amount.certainty(), CostCertainty::Estimated);
  }
}
