use serde::Serialize;

use crate::api::types::chat_types::chat_role::ChatRole;

/// One message in a chat conversation. Serializes as `{"role": "...", "content": "..."}`.
#[derive(Clone, Debug, Serialize)]
pub struct ChatMessage {
  pub role: ChatRole,
  pub content: String,
}

impl ChatMessage {
  pub fn system(content: impl Into<String>) -> Self {
    Self { role: ChatRole::System, content: content.into() }
  }
  pub fn user(content: impl Into<String>) -> Self {
    Self { role: ChatRole::User, content: content.into() }
  }
  pub fn assistant(content: impl Into<String>) -> Self {
    Self { role: ChatRole::Assistant, content: content.into() }
  }

  /// Rough token estimate for cost pre-calculation (~4 chars/token, +4 overhead per message).
  pub(crate) fn estimated_tokens(&self) -> u64 {
    (self.content.chars().count() as u64).div_ceil(4) + 4
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn serializes_role_and_content() {
    let m = ChatMessage::user("hello");
    let json = serde_json::to_string(&m).unwrap();
    assert_eq!(json, r#"{"role":"user","content":"hello"}"#);
  }

  #[test]
  fn token_estimate_is_positive() {
    assert!(ChatMessage::user("hello world this is a test").estimated_tokens() >= 5);
  }
}
