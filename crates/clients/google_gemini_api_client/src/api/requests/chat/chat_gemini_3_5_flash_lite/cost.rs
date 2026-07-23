use client_core::{CostCalculator, CostCertainty, Micros};

use crate::api::requests::chat::chat_gemini_3_5_flash_lite::chat_gemini_3_5_flash_lite::Gemini35FlashLiteRequest;
use crate::api::requests::chat::common::estimate_chat_micros;

// Gemini 3.5 Flash-Lite (gemini-3.5-flash-lite): $0.30 / 1M input tokens · $2.50 / 1M output tokens (thinking included).
const MICROS_PER_MTOK_IN: i64 = 300_000;
const MICROS_PER_MTOK_OUT: i64 = 2_500_000;

impl CostCalculator for Gemini35FlashLiteRequest {
  fn cost_micros(&self) -> Micros {
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

  fn req(prompt: &str, max_out: Option<u32>) -> Gemini35FlashLiteRequest {
    Gemini35FlashLiteRequest {
      messages: vec![GeminiChatMessage::user(prompt)],
      system_instruction: None,
      temperature: None,
      top_p: None,
      max_output_tokens: max_out,
      thinking_level: None,
      tools: vec![],
      response_json_schema: None,
      previous_interaction_id: None,
    }
  }

  #[test]
  fn exact_estimate_for_known_inputs() {
    // "hello world" = 11 chars → ceil(11/4)+4 = 7 input tokens; 1000 output tokens.
    let expected = 7 * MICROS_PER_MTOK_IN / 1_000_000 + 1000 * MICROS_PER_MTOK_OUT / 1_000_000;
    assert_eq!(req("hello world", Some(1000)).cost_micros(), Micros(expected));
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
