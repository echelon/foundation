use client_core::{CostCalculator, Micros};

use crate::api::requests::chat::chat_completion::chat_completion::ChatCompletionRequest;

/// Default assumed output length when `max_output_tokens` is unset — chat cost can only be
/// ESTIMATED pre-call (the real output length is unknown until the response). We assume the
/// caller's `max_output_tokens` as the output ceiling, or this default.
const DEFAULT_ASSUMED_OUTPUT_TOKENS: u64 = 512;

/// Estimated cost = (est. input tokens × input rate) + (assumed output tokens × output rate),
/// each per-1M-token rate from the model. Input tokens are estimated from message text
/// (~4 chars/token). Authoritative cost comes from the response `usage` post-call.
impl CostCalculator for ChatCompletionRequest {
  fn cost_micros(&self) -> Micros {
    let (in_rate, out_rate) = self.model.pricing_micros_per_mtok();

    let input_tokens: u64 = self.messages.iter().map(|m| m.estimated_tokens()).sum();
    let output_tokens = self.max_output_tokens.map(u64::from).unwrap_or(DEFAULT_ASSUMED_OUTPUT_TOKENS);

    let input_micros = (input_tokens as i64) * in_rate / 1_000_000;
    let output_micros = (output_tokens as i64) * out_rate / 1_000_000;
    Micros(input_micros + output_micros)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::api::types::chat_types::chat_message::ChatMessage;
  use crate::api::types::chat_types::chat_model::ChatModel;

  fn req(model: ChatModel, prompt: &str, max_out: Option<u32>) -> ChatCompletionRequest {
    ChatCompletionRequest {
      model,
      messages: vec![ChatMessage::user(prompt)],
      max_output_tokens: max_out,
      temperature: None,
      top_p: None,
      user: None,
    }
  }

  #[test]
  fn cost_is_positive_and_uses_max_tokens() {
    let c = req(ChatModel::Gpt5_5, "hello world", Some(1000)).cost_micros();
    assert!(c.0 > 0);
  }

  #[test]
  fn more_output_tokens_costs_more() {
    let small = req(ChatModel::Gpt5_5, "hi", Some(100)).cost_micros();
    let big = req(ChatModel::Gpt5_5, "hi", Some(4000)).cost_micros();
    assert!(big.0 > small.0);
  }

  #[test]
  fn cheaper_model_costs_less() {
    let pro = req(ChatModel::Gpt5_5Pro, "some prompt text here", Some(1000)).cost_micros();
    let mini = req(ChatModel::Gpt5Mini, "some prompt text here", Some(1000)).cost_micros();
    assert!(mini.0 < pro.0);
  }

  #[test]
  fn longer_prompt_costs_at_least_as_much() {
    let short = req(ChatModel::Gpt5, "hi", Some(500)).cost_micros();
    let long = req(ChatModel::Gpt5, &"word ".repeat(500), Some(500)).cost_micros();
    assert!(long.0 >= short.0);
  }

  #[test]
  fn unset_max_tokens_uses_default_assumption() {
    let unset = req(ChatModel::Gpt5, "hi", None).cost_micros();
    let set_default = req(ChatModel::Gpt5, "hi", Some(DEFAULT_ASSUMED_OUTPUT_TOKENS as u32)).cost_micros();
    assert_eq!(unset, set_default);
  }
}
