/// `reasoning.effort` for the GPT-5 family. Supported values are
/// MODEL-DEPENDENT (check the model page); unsupported values are rejected by
/// the API. `Medium` is the usual default.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ReasoningEffort {
  /// No reasoning pass at all (where the model allows it).
  None,
  Minimal,
  Low,
  Medium,
  High,
  XHigh,
  Max,
}

impl ReasoningEffort {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::None => "none",
      Self::Minimal => "minimal",
      Self::Low => "low",
      Self::Medium => "medium",
      Self::High => "high",
      Self::XHigh => "xhigh",
      Self::Max => "max",
    }
  }
}

impl serde::Serialize for ReasoningEffort {
  fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(self.as_str())
  }
}
