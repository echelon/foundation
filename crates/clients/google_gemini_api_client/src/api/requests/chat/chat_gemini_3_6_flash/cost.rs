
use crate::api::requests::chat::chat_gemini_3_6_flash::chat_gemini_3_6_flash::Gemini36FlashRequest;

// Gemini 3.6 Flash (gemini-3.6-flash): $1.50 / 1M input tokens · $7.50 / 1M output tokens (thinking included).
const MICROS_PER_MTOK_IN: i64 = 1_500_000;
const MICROS_PER_MTOK_OUT: i64 = 7_500_000;




use client_core::{CostCalculator, CostBreakdown, CostComponent, CostFormula, CostQuantity, CostUnit, RateRow};

use crate::api::requests::chat::common::DEFAULT_ASSUMED_OUTPUT_TOKENS as ASSUMED_OUT;

impl CostCalculator for Gemini36FlashRequest {
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
    // "hello world" = 7 est. input tokens; 1000 output tokens at the model rates.
    let expected = 7 * MICROS_PER_MTOK_IN / 1_000_000 + 1000 * MICROS_PER_MTOK_OUT / 1_000_000;
    let diff = (breakdown.amount.micros().0 - expected).abs();
    assert!(diff <= 1, "diff {diff}"); // ≤1 micro of rounding-order drift
  }
}
