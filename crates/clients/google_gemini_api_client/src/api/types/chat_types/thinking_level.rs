use serde::Serialize;

/// `generation_config.thinking_level` — how much internal reasoning the model
/// spends before answering. Supported values vary by model generation; the
/// Gemini 3.x text models document `low`/`high` (image models add `minimal`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ThinkingLevel {
  Minimal,
  Low,
  High,
}

impl ThinkingLevel {
  /// Wire representation.
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Minimal => "minimal",
      Self::Low => "low",
      Self::High => "high",
    }
  }
}

impl Serialize for ThinkingLevel {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(self.as_str())
  }
}
