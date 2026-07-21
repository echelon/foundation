/// How closely edits preserve the input image (`input_fidelity` on the image tool).
/// Supported by GPT Image 1 / 1 Mini / 1.5 — NOT by GPT Image 2 (its request struct simply
/// has no such field, so the type system enforces it).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum InputFidelity {
  Low,
  High,
}

impl InputFidelity {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Low => "low",
      Self::High => "high",
    }
  }
}

impl serde::Serialize for InputFidelity {
  fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(self.as_str())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn wire_strings() {
    assert_eq!(InputFidelity::Low.as_str(), "low");
    assert_eq!(serde_json::to_string(&InputFidelity::High).unwrap(), "\"high\"");
  }
}
