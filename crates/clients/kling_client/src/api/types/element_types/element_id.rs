use serde::Deserialize;

/// An element id. Kling's docs are inconsistent about the wire type: create
/// requests and delete bodies take it as a **string**, but query responses
/// show a bare **number** — so the inbound side accepts either (the untagged
/// number-or-string pattern; never `serde_json::Value`).
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum ElementId {
  Number(i64),
  Text(String),
}

impl ElementId {
  /// The canonical string form — what outbound requests (video `contents`
  /// element refs, delete bodies) send.
  pub fn as_string(&self) -> String {
    match self {
      Self::Number(n) => n.to_string(),
      Self::Text(s) => s.clone(),
    }
  }
}

impl std::fmt::Display for ElementId {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Number(n) => write!(f, "{n}"),
      Self::Text(s) => write!(f, "{s}"),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn accepts_both_wire_types() {
    let number: ElementId = serde_json::from_str("829836802793406551").unwrap();
    let text: ElementId = serde_json::from_str(r#""829836802793406551""#).unwrap();
    assert_eq!(number.as_string(), "829836802793406551");
    assert_eq!(text.as_string(), "829836802793406551");
  }
}
