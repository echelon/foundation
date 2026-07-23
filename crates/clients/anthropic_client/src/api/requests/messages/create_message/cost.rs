use client_core::{CostCalculator, MicroUsd};

use crate::api::requests::messages::create_message::create_message::CreateMessageRequest;

/// Estimated cost = (est. input tokens × input rate) + (max_tokens × output rate), per-1M
/// rates from the model. Output length is unknown pre-call, so `max_tokens` is the ceiling;
/// the response `usage` is authoritative for the real bill.
impl CostCalculator for CreateMessageRequest {
  fn cost_micros(&self) -> MicroUsd {
    let (in_rate, out_rate) = self.model.pricing_micros_per_mtok();

    let mut input_tokens: u64 = self.messages.iter().map(|m| m.estimated_tokens()).sum();
    if let Some(system) = &self.system {
      input_tokens += (system.chars().count() as u64).div_ceil(4);
    }
    let output_tokens = self.max_tokens as u64;

    let input_micros = (input_tokens as i64) * in_rate / 1_000_000;
    let output_micros = (output_tokens as i64) * out_rate / 1_000_000;
    MicroUsd(input_micros + output_micros)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::api::types::chat_types::anthropic_model::AnthropicModel;
  use crate::api::types::chat_types::message::Message;

  fn req(model: AnthropicModel, prompt: &str, max_tokens: u32) -> CreateMessageRequest {
    CreateMessageRequest::new(model, vec![Message::user(prompt)], max_tokens)
  }

  #[test]
  fn cost_positive() {
    assert!(req(AnthropicModel::ClaudeOpus4_8, "hello", 1000).cost_micros().0 > 0);
  }

  #[test]
  fn opus_costs_more_than_haiku() {
    let opus = req(AnthropicModel::ClaudeOpus4_8, "some prompt", 1000).cost_micros();
    let haiku = req(AnthropicModel::ClaudeHaiku4_5, "some prompt", 1000).cost_micros();
    assert!(opus.0 > haiku.0);
  }

  #[test]
  fn more_max_tokens_costs_more() {
    let small = req(AnthropicModel::ClaudeSonnet5, "hi", 100).cost_micros();
    let big = req(AnthropicModel::ClaudeSonnet5, "hi", 8000).cost_micros();
    assert!(big.0 > small.0);
  }

  #[test]
  fn system_prompt_adds_input_cost() {
    let base = req(AnthropicModel::ClaudeOpus4_8, "hi", 1000).cost_micros();
    let with_sys = req(AnthropicModel::ClaudeOpus4_8, "hi", 1000)
      .with_system(&"context ".repeat(200))
      .cost_micros();
    assert!(with_sys.0 > base.0);
  }

  #[test]
  fn opus_1m_output_tokens_is_25_usd() {
    // 1,000,000 output tokens × $25/MTok = $25 = 25_000_000 micros (input ~ negligible here).
    let r = req(AnthropicModel::ClaudeOpus4_8, "", 1_000_000);
    // input is tiny (one empty-ish user message); output dominates.
    assert!(r.cost_micros().0 >= 25_000_000);
    assert!(r.cost_micros().0 < 25_100_000);
  }
}
