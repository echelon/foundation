/// Output size for GPT Image. `Auto` lets OpenAI choose (the default for GPT models).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ImageSize {
  /// 1024x1024 (square).
  Square1024,
  /// 1536x1024 (landscape).
  Landscape1536x1024,
  /// 1024x1536 (portrait).
  Portrait1024x1536,
  /// Let the model choose.
  Auto,
}

impl ImageSize {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Square1024 => "1024x1024",
      Self::Landscape1536x1024 => "1536x1024",
      Self::Portrait1024x1536 => "1024x1536",
      Self::Auto => "auto",
    }
  }
}

impl serde::Serialize for ImageSize {
  fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(self.as_str())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn wire_strings() {
    assert_eq!(ImageSize::Square1024.as_str(), "1024x1024");
    assert_eq!(ImageSize::Landscape1536x1024.as_str(), "1536x1024");
    assert_eq!(ImageSize::Auto.as_str(), "auto");
  }

  #[test]
  fn serializes_as_string() {
    assert_eq!(serde_json::to_string(&ImageSize::Portrait1024x1536).unwrap(), "\"1024x1536\"");
  }
}
