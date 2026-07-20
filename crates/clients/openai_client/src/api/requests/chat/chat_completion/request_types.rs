use serde::{Deserialize, Serialize};

use crate::api::types::chat_types::chat_message::ChatMessage;

// ── Wire request ──

#[derive(Serialize, Debug)]
pub(crate) struct ChatCompletionRequestBody {
  pub model: String,
  pub messages: Vec<ChatMessage>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub max_completion_tokens: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub temperature: Option<f32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub top_p: Option<f32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub user: Option<String>,
}

// ── Wire response ──

#[derive(Deserialize, Debug)]
pub(crate) struct ChatCompletionResponseBody {
  pub id: String,
  pub model: String,
  pub choices: Vec<ChatChoice>,
  pub usage: Option<ChatUsageWire>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct ChatChoice {
  pub message: ChatResponseMessage,
  pub finish_reason: Option<String>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct ChatResponseMessage {
  #[allow(dead_code)]
  pub role: String,
  pub content: Option<String>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct ChatUsageWire {
  #[serde(default)]
  pub prompt_tokens: i64,
  #[serde(default)]
  pub completion_tokens: i64,
  #[serde(default)]
  pub total_tokens: i64,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn response_deserializes() {
    let json = r#"{
      "id": "chatcmpl-1",
      "model": "gpt-5.5",
      "choices": [ { "message": { "role": "assistant", "content": "Hi there" }, "finish_reason": "stop" } ],
      "usage": { "prompt_tokens": 9, "completion_tokens": 3, "total_tokens": 12 }
    }"#;
    let parsed: ChatCompletionResponseBody = serde_json::from_str(json).unwrap();
    assert_eq!(parsed.choices[0].message.content.as_deref(), Some("Hi there"));
    assert_eq!(parsed.choices[0].finish_reason.as_deref(), Some("stop"));
    assert_eq!(parsed.usage.unwrap().total_tokens, 12);
  }
}
