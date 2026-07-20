/// A chat message role. Wire form is the lowercase string.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ChatRole {
  System,
  User,
  Assistant,
  Developer,
}

impl ChatRole {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::System => "system",
      Self::User => "user",
      Self::Assistant => "assistant",
      Self::Developer => "developer",
    }
  }
}

impl serde::Serialize for ChatRole {
  fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(self.as_str())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn wire_strings() {
    assert_eq!(ChatRole::User.as_str(), "user");
    assert_eq!(ChatRole::Assistant.as_str(), "assistant");
    assert_eq!(serde_json::to_string(&ChatRole::System).unwrap(), "\"system\"");
  }
}
