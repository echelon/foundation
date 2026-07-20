/// A message role in the Anthropic Messages API. Only `user` and `assistant` appear in the
/// `messages` array (the system prompt is a separate top-level field).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MessageRole {
  User,
  Assistant,
}

impl MessageRole {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::User => "user",
      Self::Assistant => "assistant",
    }
  }
}

impl serde::Serialize for MessageRole {
  fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(self.as_str())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn wire_strings() {
    assert_eq!(MessageRole::User.as_str(), "user");
    assert_eq!(serde_json::to_string(&MessageRole::Assistant).unwrap(), "\"assistant\"");
  }
}
