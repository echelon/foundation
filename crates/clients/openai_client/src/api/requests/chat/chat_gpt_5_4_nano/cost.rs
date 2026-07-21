use client_core::{CostCalculator, Micros};

use crate::api::requests::chat::common::estimate_chat_micros;
use crate::api::requests::chat::chat_gpt_5_4_nano::chat_gpt_5_4_nano::ChatGpt5_4NanoRequest;
use crate::api::types::chat_types::chat_model::ChatModel;

/// ESTIMATE: input tokens from message text (~4 chars/token) at gpt-5.4-nano's published rates,
/// output assumed at `max_output_tokens` (or the shared default). Authoritative cost is the
/// response `usage` post-call.
impl CostCalculator for ChatGpt5_4NanoRequest {
  fn cost_micros(&self) -> Micros {
    estimate_chat_micros(&ChatModel::Gpt5_4Nano, &self.messages, self.max_output_tokens)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::api::requests::chat::common::DEFAULT_ASSUMED_OUTPUT_TOKENS;
  use crate::api::types::chat_types::chat_message::ChatMessage;

  fn req(prompt: &str, max_out: Option<u32>) -> ChatGpt5_4NanoRequest {
    let mut r = ChatGpt5_4NanoRequest::new(vec![ChatMessage::user(prompt)]);
    r.max_output_tokens = max_out;
    r
  }

  #[test]
  fn exact_estimate_for_known_inputs() {
    // "hello world" = 11 chars → ceil(11/4)+4 = 7 input tokens; 1000 output tokens.
    let (in_rate, out_rate) = ChatModel::Gpt5_4Nano.pricing_micros_per_mtok();
    let expected = 7 * in_rate / 1_000_000 + 1000 * out_rate / 1_000_000;
    assert_eq!(req("hello world", Some(1000)).cost_micros(), Micros(expected));
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
