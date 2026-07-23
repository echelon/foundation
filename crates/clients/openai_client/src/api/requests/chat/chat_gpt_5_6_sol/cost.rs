use client_core::{CostCalculator, CostBreakdown};

use crate::api::requests::chat::chat_gpt_5_6_sol::chat_gpt_5_6_sol::ChatGpt5_6SolRequest;
use crate::api::requests::chat::common::chat_cost_breakdown;
use crate::api::types::chat_types::chat_model::ChatModel;

/// ESTIMATE: input tokens from message text (~4 chars/token) at the model's
/// published rates, output assumed at `max_output_tokens` (or the shared
/// default). Authoritative cost is the response `usage` post-call.
impl CostCalculator for ChatGpt5_6SolRequest {
  fn estimate_cost(&self) -> CostBreakdown {
    chat_cost_breakdown(&ChatModel::Gpt5_6Sol, &self.messages, self.max_output_tokens)
  }
}

#[cfg(test)]
mod tests {
  use client_core::CostCertainty;

  use super::*;
  use crate::api::requests::chat::common::DEFAULT_ASSUMED_OUTPUT_TOKENS;
  use crate::api::types::chat_types::chat_message::ChatMessage;

  fn req(prompt: &str, max_out: Option<u32>) -> ChatGpt5_6SolRequest {
    let mut r = ChatGpt5_6SolRequest::new(vec![ChatMessage::user(prompt)]);
    r.max_output_tokens = max_out;
    r
  }

  #[test]
  fn exact_estimate_for_known_inputs() {
    // "hello world" = 11 chars → ceil(11/4)+4 = 7 input tokens; 1000 output tokens.
    let (in_rate, out_rate) = ChatModel::Gpt5_6Sol.pricing_micros_per_mtok();
    let expected = 7 * in_rate / 1_000_000 + 1000 * out_rate / 1_000_000;
    let breakdown = req("hello world", Some(1000)).estimate_cost();
    let diff = (breakdown.amount.micros().0 - expected).abs();
    assert!(diff <= 1, "diff {diff}"); // ≤1 micro of rounding-order drift
    assert_eq!(breakdown.amount.certainty(), CostCertainty::Estimated);
    assert_eq!(breakdown.components.len(), 2);
  }

  #[test]
  fn more_output_costs_more_and_default_applies() {
    assert!(
      req("hi", Some(4000)).estimate_cost().amount.micros() > req("hi", Some(100)).estimate_cost().amount.micros()
    );
    assert_eq!(
      req("hi", None).estimate_cost().amount.micros(),
      req("hi", Some(DEFAULT_ASSUMED_OUTPUT_TOKENS as u32)).estimate_cost().amount.micros()
    );
  }
}
