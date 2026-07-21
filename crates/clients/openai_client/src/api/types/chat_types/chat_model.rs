/// OpenAI LLMs this crate binds (each has its own binding fn under `api/requests/chat`).
/// `Custom` is the forward-compat escape hatch (used e.g. as an image-tool driver override).
/// Each variant carries published per-million-token pricing for the cost ESTIMATORS.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatModel {
  /// `gpt-5.6-sol` (alias `gpt-5.6`) — the flagship.
  Gpt5_6Sol,
  /// `gpt-5.5`
  Gpt5_5,
  /// `gpt-5.4-nano` — the cheap/fast tier.
  Gpt5_4Nano,
  /// Any other identifier.
  Custom(String),
}

impl ChatModel {
  pub fn as_str(&self) -> &str {
    match self {
      Self::Gpt5_6Sol => "gpt-5.6-sol",
      Self::Gpt5_5 => "gpt-5.5",
      Self::Gpt5_4Nano => "gpt-5.4-nano",
      Self::Custom(s) => s.as_str(),
    }
  }

  /// (input, output) price in **micro-USD per 1,000,000 tokens** — published July 2026
  /// rates. ESTIMATE ONLY: gpt-5.6-sol doubles above a 272K-token context breakpoint, and
  /// caching discounts apply; the response `usage` is authoritative. Custom models assume
  /// the flagship rate.
  pub fn pricing_micros_per_mtok(&self) -> (i64, i64) {
    match self {
      Self::Gpt5_6Sol => (5_000_000, 30_000_000),  // $5.00 / $30.00
      Self::Gpt5_5 => (5_000_000, 30_000_000),     // $5.00 / $30.00
      Self::Gpt5_4Nano => (200_000, 1_250_000),    // $0.20 / $1.25
      Self::Custom(_) => (5_000_000, 30_000_000),
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
    assert_eq!(ChatModel::Gpt5_6Sol.as_str(), "gpt-5.6-sol");
    assert_eq!(ChatModel::Gpt5_5.as_str(), "gpt-5.5");
    assert_eq!(ChatModel::Gpt5_4Nano.as_str(), "gpt-5.4-nano");
  }

  #[test]
  fn pricing_is_output_ge_input() {
    for m in [ChatModel::Gpt5_6Sol, ChatModel::Gpt5_5, ChatModel::Gpt5_4Nano] {
      let (i, o) = m.pricing_micros_per_mtok();
      assert!(o >= i, "output should cost >= input for {}", m.as_str());
    }
  }

  #[test]
  fn nano_is_the_cheap_tier() {
    assert!(ChatModel::Gpt5_4Nano.pricing_micros_per_mtok().0 < ChatModel::Gpt5_5.pricing_micros_per_mtok().0);
  }

  #[test]
  fn serializes_as_string() {
    assert_eq!(serde_json::to_string(&ChatModel::Gpt5_6Sol).unwrap(), "\"gpt-5.6-sol\"");
  }
}
