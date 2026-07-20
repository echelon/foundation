/// GPT Image models accepted by `/v1/images/generations` and `/v1/images/edits`.
///
/// `Custom` is an escape hatch for ids not yet enumerated (OpenAI ships models faster
/// than this crate releases). Serializes as the exact wire string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageModel {
  /// `gpt-image-1` — the original GPT Image model.
  GptImage1,
  /// `gpt-image-1-mini` — smaller/cheaper GPT Image 1.
  GptImage1Mini,
  /// `gpt-image-1.5` — faster, cheaper I/O than v1.
  GptImage1_5,
  /// `gpt-image-2` — reasoning-based, flexible resolutions, 4K.
  GptImage2,
  /// Any other identifier (forward/back compat).
  Custom(String),
}

impl ImageModel {
  pub fn as_str(&self) -> &str {
    match self {
      Self::GptImage1 => "gpt-image-1",
      Self::GptImage1Mini => "gpt-image-1-mini",
      Self::GptImage1_5 => "gpt-image-1.5",
      Self::GptImage2 => "gpt-image-2",
      Self::Custom(s) => s.as_str(),
    }
  }
}

impl serde::Serialize for ImageModel {
  fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(self.as_str())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn wire_strings() {
    assert_eq!(ImageModel::GptImage1.as_str(), "gpt-image-1");
    assert_eq!(ImageModel::GptImage1_5.as_str(), "gpt-image-1.5");
    assert_eq!(ImageModel::GptImage2.as_str(), "gpt-image-2");
  }

  #[test]
  fn custom_passthrough() {
    assert_eq!(ImageModel::Custom("gpt-image-3".into()).as_str(), "gpt-image-3");
  }

  #[test]
  fn serializes_as_plain_string() {
    assert_eq!(serde_json::to_string(&ImageModel::GptImage2).unwrap(), "\"gpt-image-2\"");
  }
}
