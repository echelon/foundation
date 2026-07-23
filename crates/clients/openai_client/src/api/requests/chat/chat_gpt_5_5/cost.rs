use client_core::{CostCalculator, MicroUsd};

use crate::api::requests::chat::common::estimate_chat_micros;
use crate::api::requests::chat::chat_gpt_5_5::chat_gpt_5_5::ChatGpt5_5Request;
use crate::api::types::chat_types::chat_model::ChatModel;

/// ESTIMATE: input tokens from message text (~4 chars/token) at gpt-5.5's published rates,
/// output assumed at `max_output_tokens` (or the shared default). Authoritative cost is the
/// response `usage` post-call.
impl CostCalculator for ChatGpt5_5Request {
  fn cost_micros(&self) -> MicroUsd {
    estimate_chat_micros(&ChatModel::Gpt5_5, &self.messages, self.max_output_tokens)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::api::requests::chat::common::DEFAULT_ASSUMED_OUTPUT_TOKENS;
  use crate::api::types::chat_types::chat_message::ChatMessage;

  fn req(prompt: &str, max_out: Option<u32>) -> ChatGpt5_5Request {
    let mut r = ChatGpt5_5Request::new(vec![ChatMessage::user(prompt)]);
    r.max_output_tokens = max_out;
    r
  }

  #[test]
  fn exact_estimate_for_known_inputs() {
    // "hello world" = 11 chars → ceil(11/4)+4 = 7 input tokens; 1000 output tokens.
    let (in_rate, out_rate) = ChatModel::Gpt5_5.pricing_micros_per_mtok();
    let expected = 7 * in_rate / 1_000_000 + 1000 * out_rate / 1_000_000;
    assert_eq!(req("hello world", Some(1000)).cost_micros(), MicroUsd(expected));
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
