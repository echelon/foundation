use client_core::{CostCalculator, CostBreakdown, CostComponent, CostFormula, CostQuantity, CostUnit, RateRow};

use crate::api::requests::messages::create_message::create_message::CreateMessageRequest;

/// Estimated cost = (est. input tokens × input rate) + (max_tokens × output rate), per-1M
/// rates from the model. Output length is unknown pre-call, so `max_tokens` is the ceiling;
/// the response `usage` is authoritative for the real bill.
impl CostCalculator for CreateMessageRequest {
  fn estimate_cost(&self) -> CostBreakdown {
    let (in_rate, out_rate) = self.model.pricing_micros_per_mtok();

    let mut input_tokens: u64 = self.messages.iter().map(|m| m.estimated_tokens()).sum();
    if let Some(system) = &self.system {
      input_tokens += (system.chars().count() as u64).div_ceil(4);
    }
    let output_tokens = self.max_tokens as u64;

    CostBreakdown::from_components(
      CostFormula {
        display: "input_tokens × rate + max_tokens × rate (output ceiling)",
        rates: vec![
          RateRow {
            dimension: "tokens",
            value: "input".to_string(),
            unit: CostUnit::PerMillionInputTokens,
            price_micros: in_rate,
          },
          RateRow {
            dimension: "tokens",
            value: "output".to_string(),
            unit: CostUnit::PerMillionOutputTokens,
            price_micros: out_rate,
          },
        ],
      },
      vec![
        CostComponent::new(
          "Input tokens (~4 chars/token estimate)",
          CostUnit::PerMillionInputTokens,
          in_rate,
          CostQuantity::Estimated(input_tokens as f64),
        ),
        CostComponent::new(
          "Output tokens (at max_tokens ceiling)",
          CostUnit::PerMillionOutputTokens,
          out_rate,
          CostQuantity::Estimated(output_tokens as f64),
        )
        .with_note("output length unknown pre-call; response usage is authoritative"),
      ],
      true,
    )
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
    assert!(req(AnthropicModel::ClaudeOpus4_8, "hello", 1000).estimate_cost().amount.micros().0 > 0);
  }

  #[test]
  fn opus_costs_more_than_haiku() {
    let opus = req(AnthropicModel::ClaudeOpus4_8, "some prompt", 1000).estimate_cost().amount.micros();
    let haiku = req(AnthropicModel::ClaudeHaiku4_5, "some prompt", 1000).estimate_cost().amount.micros();
    assert!(opus.0 > haiku.0);
  }

  #[test]
  fn more_max_tokens_costs_more() {
    let small = req(AnthropicModel::ClaudeSonnet5, "hi", 100).estimate_cost().amount.micros();
    let big = req(AnthropicModel::ClaudeSonnet5, "hi", 8000).estimate_cost().amount.micros();
    assert!(big.0 > small.0);
  }

  #[test]
  fn system_prompt_adds_input_cost() {
    let base = req(AnthropicModel::ClaudeOpus4_8, "hi", 1000).estimate_cost().amount.micros();
    let with_sys = req(AnthropicModel::ClaudeOpus4_8, "hi", 1000)
      .with_system(&"context ".repeat(200))
      .estimate_cost().amount.micros();
    assert!(with_sys.0 > base.0);
  }

  #[test]
  fn opus_1m_output_tokens_is_25_usd() {
    // 1,000,000 output tokens × $25/MTok = $25 = 25_000_000 micros (input ~ negligible here).
    let r = req(AnthropicModel::ClaudeOpus4_8, "", 1_000_000);
    // input is tiny (one empty-ish user message); output dominates.
    assert!(r.estimate_cost().amount.micros().0 >= 25_000_000);
    assert!(r.estimate_cost().amount.micros().0 < 25_100_000);
  }
}
