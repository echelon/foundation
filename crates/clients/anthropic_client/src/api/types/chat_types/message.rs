use serde::Serialize;

use crate::api::types::chat_types::message_role::MessageRole;

/// One message in the conversation. Content is sent as a plain string (Anthropic also
/// accepts a content-block array; the string form is the common case).
#[derive(Clone, Debug, Serialize)]
pub struct Message {
  pub role: MessageRole,
  pub content: String,
}

impl Message {
  pub fn user(content: impl Into<String>) -> Self {
    Self { role: MessageRole::User, content: content.into() }
  }
  pub fn assistant(content: impl Into<String>) -> Self {
    Self { role: MessageRole::Assistant, content: content.into() }
  }

  /// Rough token estimate for cost pre-calculation (~4 chars/token, +4 overhead).
  pub(crate) fn estimated_tokens(&self) -> u64 {
    (self.content.chars().count() as u64).div_ceil(4) + 4
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn serializes() {
    let m = Message::user("hello");
    assert_eq!(serde_json::to_string(&m).unwrap(), r#"{"role":"user","content":"hello"}"#);
  }
}
