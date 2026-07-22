use client_core::{CostCalculator, CostCertainty, Micros};

use crate::api::requests::videos::generate_gemini_omni_flash::generate_gemini_omni_flash::GeminiOmniFlashRequest;

// Gemini Omni Flash (gemini-omni-flash-preview): TOKEN-billed — $17.50 / 1M
// video output tokens ≈ $0.10 per second of 720p video (plus $9.00 / 1M text
// output tokens, negligible here). Output length is prompt-driven with no
// duration parameter, so the price can only be ESTIMATED up front; the
// provider's reported usage is authoritative after the fact.

const MICROS_PER_ESTIMATED_SECOND: i64 = 100_000;
const DEFAULT_ESTIMATED_SECONDS: i64 = 8;

impl CostCalculator for GeminiOmniFlashRequest {
  fn cost_micros(&self) -> Micros {
    let seconds = self.expected_output_seconds.unwrap_or(DEFAULT_ESTIMATED_SECONDS);
    Micros(MICROS_PER_ESTIMATED_SECOND * seconds)
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
    assert_eq!(req(None).cost_micros(), Micros(800_000)); // 8 s default ≈ $0.80
    assert_eq!(req(Some(4)).cost_micros(), Micros(400_000));
    assert_eq!(req(None).cost_certainty(), CostCertainty::Estimated);
  }
}
