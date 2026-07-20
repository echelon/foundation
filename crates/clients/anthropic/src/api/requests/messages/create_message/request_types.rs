use serde::{Deserialize, Serialize};

use crate::api::types::chat_types::message::Message;

// ── Wire request ──

#[derive(Serialize, Debug)]
pub(crate) struct CreateMessageRequestBody {
  pub model: String,
  pub max_tokens: u32,
  pub messages: Vec<Message>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub system: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub temperature: Option<f32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub top_p: Option<f32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub stop_sequences: Option<Vec<String>>,
}

// ── Wire response ──

#[derive(Deserialize, Debug)]
pub(crate) struct CreateMessageResponseBody {
  pub id: String,
  pub model: String,
  pub content: Vec<ContentBlock>,
  pub stop_reason: Option<String>,
  pub usage: Option<MessageUsageWire>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct ContentBlock {
  #[serde(rename = "type")]
  #[allow(dead_code)]
  pub block_type: String,
  pub text: Option<String>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct MessageUsageWire {
  #[serde(default)]
  pub input_tokens: i64,
  #[serde(default)]
  pub output_tokens: i64,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn response_deserializes_text_blocks() {
    let json = r#"{
      "id": "msg_1",
      "type": "message",
      "role": "assistant",
      "model": "claude-opus-4-8",
      "content": [ { "type": "text", "text": "Hello, " }, { "type": "text", "text": "world" } ],
      "stop_reason": "end_turn",
      "usage": { "input_tokens": 10, "output_tokens": 5 }
    }"#;
    let parsed: CreateMessageResponseBody = serde_json::from_str(json).unwrap();
    let text: String = parsed.content.iter().filter_map(|b| b.text.clone()).collect();
    assert_eq!(text, "Hello, world");
    assert_eq!(parsed.stop_reason.as_deref(), Some("end_turn"));
    assert_eq!(parsed.usage.unwrap().output_tokens, 5);
  }
}
