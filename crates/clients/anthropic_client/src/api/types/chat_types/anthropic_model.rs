/// Anthropic Claude models for the Messages API. `Custom` is the forward-compat escape
/// hatch (real deployments often pin a dated snapshot id via `Custom`). Each variant
/// carries published per-million-token pricing used by the cost estimator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnthropicModel {
  /// `claude-opus-4-8` — frontier reasoning/coding.
  ClaudeOpus4_8,
  /// `claude-sonnet-5`
  ClaudeSonnet5,
  /// `claude-haiku-4-5` — fast/cheap.
  ClaudeHaiku4_5,
  /// `claude-fable-5`
  ClaudeFable5,
  /// Any other identifier (e.g. a dated snapshot `claude-opus-4-8-20260501`).
  Custom(String),
}

impl AnthropicModel {
  pub fn as_str(&self) -> &str {
    match self {
      Self::ClaudeOpus4_8 => "claude-opus-4-8",
      Self::ClaudeSonnet5 => "claude-sonnet-5",
      Self::ClaudeHaiku4_5 => "claude-haiku-4-5",
      Self::ClaudeFable5 => "claude-fable-5",
      Self::Custom(s) => s.as_str(),
    }
  }

  /// (input, output) price in **micro-USD per 1,000,000 tokens**.
  pub fn pricing_micros_per_mtok(&self) -> (i64, i64) {
    match self {
      Self::ClaudeOpus4_8 => (5_000_000, 25_000_000),
      Self::ClaudeSonnet5 => (3_000_000, 15_000_000),
      Self::ClaudeHaiku4_5 => (1_000_000, 5_000_000),
      Self::ClaudeFable5 => (1_000_000, 5_000_000),
      Self::Custom(_) => (3_000_000, 15_000_000),
    }
  }
}

impl serde::Serialize for AnthropicModel {
  fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(self.as_str())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn wire_strings() {
    assert_eq!(AnthropicModel::ClaudeOpus4_8.as_str(), "claude-opus-4-8");
    assert_eq!(AnthropicModel::ClaudeHaiku4_5.as_str(), "claude-haiku-4-5");
  }

  #[test]
  fn opus_pricier_than_haiku() {
    let (oi, oo) = AnthropicModel::ClaudeOpus4_8.pricing_micros_per_mtok();
    let (hi, ho) = AnthropicModel::ClaudeHaiku4_5.pricing_micros_per_mtok();
    assert!(oi > hi && oo > ho);
  }

  #[test]
  fn serializes_as_string() {
    assert_eq!(serde_json::to_string(&AnthropicModel::ClaudeSonnet5).unwrap(), "\"claude-sonnet-5\"");
  }
}
