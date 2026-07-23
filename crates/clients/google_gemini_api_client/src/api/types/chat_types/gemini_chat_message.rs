use serde::Serialize;

/// Who authored a chat turn.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GeminiChatRole {
  User,
  /// The model's own prior turns (Gemini's name for the assistant role).
  Model,
}

impl GeminiChatRole {
  /// Wire representation.
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::User => "user",
      Self::Model => "model",
    }
  }
}

impl Serialize for GeminiChatRole {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(self.as_str())
  }
}

/// One chat turn. (Server-side multi-turn via `previous_interaction_id` is the
/// API's preferred continuation mechanism; explicit message history is for
/// stateless replay.)
#[derive(Clone, Debug, Serialize)]
pub struct GeminiChatMessage {
  pub role: GeminiChatRole,
  pub text: String,
}

impl GeminiChatMessage {
  pub fn user(text: &str) -> Self {
    Self { role: GeminiChatRole::User, text: text.to_string() }
  }

  pub fn model(text: &str) -> Self {
    Self { role: GeminiChatRole::Model, text: text.to_string() }
  }

  /// Rough pre-call token estimate (~4 chars/token + small per-message overhead)
  /// for the cost calculators.
  pub fn estimated_tokens(&self) -> u64 {
    (self.text.len() as u64).div_ceil(4) + 4
  }
}
