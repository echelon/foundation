use client_core::{CostCalculator, CostCertainty, MicroUsd};

use crate::api::requests::chat::chat_gemini_3_6_flash::chat_gemini_3_6_flash::Gemini36FlashRequest;
use crate::api::requests::chat::common::estimate_chat_micros;

// Gemini 3.6 Flash (gemini-3.6-flash): $1.50 / 1M input tokens · $7.50 / 1M output tokens (thinking included).
const MICROS_PER_MTOK_IN: i64 = 1_500_000;
const MICROS_PER_MTOK_OUT: i64 = 7_500_000;

impl CostCalculator for Gemini36FlashRequest {
  fn cost_micros(&self) -> MicroUsd {
    estimate_chat_micros(
      MICROS_PER_MTOK_IN,
      MICROS_PER_MTOK_OUT,
      &self.messages,
      self.system_instruction.as_deref(),
      self.max_output_tokens,
    )
  }

  /// Token-billed (thinking tokens count as output) — always an estimate; the
  /// response usage is authoritative.
  fn cost_certainty(&self) -> CostCertainty {
    CostCertainty::Estimated
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::api::requests::chat::common::DEFAULT_ASSUMED_OUTPUT_TOKENS;
  use crate::api::types::chat_types::gemini_chat_message::GeminiChatMessage;

  fn req(prompt: &str, max_out: Option<u32>) -> Gemini36FlashRequest {
    Gemini36FlashRequest {
      messages: vec![GeminiChatMessage::user(prompt)],
      system_instruction: None,
      temperature: None,
      top_p: None,
      max_output_tokens: max_out,
      thinking_level: None,
      tools: vec![],
      response_json_schema: None,
      previous_interaction_id: None,
      store: None,
    }
  }

  #[test]
  fn exact_estimate_for_known_inputs() {
    // "hello world" = 11 chars → ceil(11/4)+4 = 7 input tokens; 1000 output tokens.
    let expected = 7 * MICROS_PER_MTOK_IN / 1_000_000 + 1000 * MICROS_PER_MTOK_OUT / 1_000_000;
    assert_eq!(req("hello world", Some(1000)).cost_micros(), MicroUsd(expected));
    assert_eq!(req("hi", None).cost_certainty(), CostCertainty::Estimated);
  }

  #[test]
  fn more_output_costs_more_and_default_applies() {
    assert!(req("hi", Some(4000)).cost_micros() > req("hi", Some(100)).cost_micros());
    assert_eq!(
      req("hi", None).cost_micros(),
      req("hi", Some(DEFAULT_ASSUMED_OUTPUT_TOKENS as u32)).cost_micros()
    );
  }
}


use client_core::{BetterCostEstimator, CostBreakdown, CostComponent, CostFormula, CostQuantity, CostUnit, RateRow};

use crate::api::requests::chat::common::DEFAULT_ASSUMED_OUTPUT_TOKENS as ASSUMED_OUT;

impl BetterCostEstimator for Gemini36FlashRequest {
  fn estimate_cost(&self) -> CostBreakdown {
    let mut input_tokens: u64 = self.messages.iter().map(|m| m.estimated_tokens()).sum();
    input_tokens += self.system_instruction.as_deref().map(|s| (s.len() as u64).div_ceil(4)).unwrap_or(0);
    let output_tokens = self.max_output_tokens.map(u64::from).unwrap_or(ASSUMED_OUT);
    CostBreakdown::from_components(
      CostFormula {
        display: "input_tokens × rate + output_tokens × rate (thinking bills as output)",
        rates: vec![
          RateRow {
            dimension: "tokens",
            value: "input".to_string(),
            unit: CostUnit::PerMillionInputTokens,
            price_micros: MICROS_PER_MTOK_IN,
          },
          RateRow {
            dimension: "tokens",
            value: "output".to_string(),
            unit: CostUnit::PerMillionOutputTokens,
            price_micros: MICROS_PER_MTOK_OUT,
          },
        ],
      },
      vec![
        CostComponent::new(
          "Input tokens (~4 chars/token estimate)",
          CostUnit::PerMillionInputTokens,
          MICROS_PER_MTOK_IN,
          CostQuantity::Estimated(input_tokens as f64),
        ),
        CostComponent::new(
          if self.max_output_tokens.is_some() { "Output tokens (at max_output_tokens cap)" } else { "Output tokens (assumed default)" },
          CostUnit::PerMillionOutputTokens,
          MICROS_PER_MTOK_OUT,
          CostQuantity::Estimated(output_tokens as f64),
        )
        .with_note("thinking tokens bill as output; response usage is authoritative"),
      ],
      true,
    )
  }
}

#[cfg(test)]
mod better_tests {
  use super::*;
  use crate::api::types::chat_types::gemini_chat_message::GeminiChatMessage;
  use client_core::CostCertainty;

  #[test]
  fn components_sum_close_to_legacy() {
    let request = Gemini36FlashRequest {
      messages: vec![GeminiChatMessage::user("hello world")],
      system_instruction: None,
      temperature: None,
      top_p: None,
      max_output_tokens: Some(1000),
      thinking_level: None,
      tools: vec![],
      response_json_schema: None,
      previous_interaction_id: None,
      store: None,
    };
    let breakdown = request.estimate_cost();
    assert_eq!(breakdown.amount.certainty(), CostCertainty::Estimated);
    assert_eq!(breakdown.components.len(), 2);
    // Same rates & token counts as the legacy estimator — totals may differ by
    // ≤1 micro from rounding order.
    let diff = (breakdown.amount.micros().0 - request.cost_micros().0).abs();
    assert!(diff <= 1, "diff {diff}");
  }
}
