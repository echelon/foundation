/// Rendering quality tier for GPT Image. Drives both the bill and the fidelity.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ImageQuality {
  Low,
  Medium,
  High,
  /// Let the model choose.
  Auto,
}

impl ImageQuality {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Low => "low",
      Self::Medium => "medium",
      Self::High => "high",
      Self::Auto => "auto",
    }
  }
}

impl serde::Serialize for ImageQuality {
  fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(self.as_str())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn wire_strings() {
    assert_eq!(ImageQuality::High.as_str(), "high");
    assert_eq!(ImageQuality::Auto.as_str(), "auto");
  }
}
