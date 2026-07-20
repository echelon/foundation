/// Output image-size tier accepted by the Gemini image endpoint's
/// `generationConfig.imageConfig.imageSize` field.
///
/// Docs: <https://ai.google.dev/gemini-api/docs/image-generation>
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ImageSize {
  /// `1K` — roughly 1 megapixel output.
  OneK,

  /// `2K` — roughly 2K output.
  TwoK,

  /// `4K` — roughly 4K output.
  FourK,
}

impl ImageSize {
  /// Wire representation — the exact string Google expects in the `imageSize`
  /// field.
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::OneK  => "1K",
      Self::TwoK  => "2K",
      Self::FourK => "4K",
    }
  }
}

// Serialize as the wire string ("1K", "2K", "4K") rather than the default
// external-tag enum form.
impl serde::Serialize for ImageSize {
  fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(self.as_str())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn maps_to_docs_strings() {
    assert_eq!(ImageSize::OneK.as_str(), "1K");
    assert_eq!(ImageSize::TwoK.as_str(), "2K");
    assert_eq!(ImageSize::FourK.as_str(), "4K");
  }

  #[test]
  fn every_variant_is_distinct() {
    let mut strs = [ImageSize::OneK.as_str(), ImageSize::TwoK.as_str(), ImageSize::FourK.as_str()];
    strs.sort();
    let mut deduped = strs.to_vec();
    deduped.dedup();
    assert_eq!(deduped.len(), 3);
  }

  #[test]
  fn serializes_as_plain_wire_string() {
    assert_eq!(serde_json::to_string(&ImageSize::OneK).unwrap(), "\"1K\"");
    assert_eq!(serde_json::to_string(&ImageSize::TwoK).unwrap(), "\"2K\"");
    assert_eq!(serde_json::to_string(&ImageSize::FourK).unwrap(), "\"4K\"");
  }
}
