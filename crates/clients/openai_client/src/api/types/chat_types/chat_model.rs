/// OpenAI chat/LLM models for `/v1/chat/completions`. `Custom` is the forward-compat
/// escape hatch. Each variant carries published per-million-token pricing used by the
/// chat cost estimator (`cost.rs`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatModel {
  /// `gpt-5.5`
  Gpt5_5,
  /// `gpt-5.5-pro`
  Gpt5_5Pro,
  /// `gpt-5.4`
  Gpt5_4,
  /// `gpt-5`
  Gpt5,
  /// `gpt-5-mini`
  Gpt5Mini,
  /// `gpt-4o`
  Gpt4o,
  /// `o4-mini`
  O4Mini,
  /// Any other identifier.
  Custom(String),
}

impl ChatModel {
  pub fn as_str(&self) -> &str {
    match self {
      Self::Gpt5_5 => "gpt-5.5",
      Self::Gpt5_5Pro => "gpt-5.5-pro",
      Self::Gpt5_4 => "gpt-5.4",
      Self::Gpt5 => "gpt-5",
      Self::Gpt5Mini => "gpt-5-mini",
      Self::Gpt4o => "gpt-4o",
      Self::O4Mini => "o4-mini",
      Self::Custom(s) => s.as_str(),
    }
  }

  /// (input, output) price in **micro-USD per 1,000,000 tokens**. Custom models default
  /// to a conservative mid-tier estimate.
  pub fn pricing_micros_per_mtok(&self) -> (i64, i64) {
    match self {
      Self::Gpt5_5 => (5_000_000, 20_000_000),
      Self::Gpt5_5Pro => (15_000_000, 60_000_000),
      Self::Gpt5_4 => (4_000_000, 16_000_000),
      Self::Gpt5 => (4_000_000, 16_000_000),
      Self::Gpt5Mini => (600_000, 2_400_000),
      Self::Gpt4o => (2_500_000, 10_000_000),
      Self::O4Mini => (1_100_000, 4_400_000),
      Self::Custom(_) => (4_000_000, 16_000_000),
    }
  }
}

impl serde::Serialize for ChatModel {
  fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(self.as_str())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn wire_strings() {
    assert_eq!(ChatModel::Gpt5_5.as_str(), "gpt-5.5");
    assert_eq!(ChatModel::Gpt5Mini.as_str(), "gpt-5-mini");
    assert_eq!(ChatModel::O4Mini.as_str(), "o4-mini");
  }

  #[test]
  fn pricing_is_output_ge_input() {
    for m in [ChatModel::Gpt5_5, ChatModel::Gpt5, ChatModel::Gpt5Mini, ChatModel::Gpt4o, ChatModel::O4Mini] {
      let (i, o) = m.pricing_micros_per_mtok();
      assert!(o >= i, "output should cost >= input for {}", m.as_str());
    }
  }

  #[test]
  fn serializes_as_string() {
    assert_eq!(serde_json::to_string(&ChatModel::Gpt5_5Pro).unwrap(), "\"gpt-5.5-pro\"");
  }
}
