/// Aspect ratios accepted by the Gemini image endpoint's
/// `generationConfig.imageConfig.aspectRatio` field.
///
/// The wire format is `"<width>:<height>"` — e.g. `Square` serialises as
/// `"1:1"`, `Landscape16x9` as `"16:9"`.
///
/// Docs: <https://ai.google.dev/gemini-api/docs/image-generation>
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ImageAspectRatio {
  /// 1:1 square.
  Square,

  /// 2:3 portrait (photo).
  Portrait2x3,

  /// 3:2 landscape (photo).
  Landscape3x2,

  /// 3:4 portrait (slight tall).
  Portrait3x4,

  /// 4:3 landscape (slight wide).
  Landscape4x3,

  /// 4:5 portrait (social).
  Portrait4x5,

  /// 5:4 landscape (social).
  Landscape5x4,

  /// 9:16 portrait (phone/vertical-video).
  Portrait9x16,

  /// 16:9 landscape (widescreen / YouTube).
  Landscape16x9,

  /// 21:9 ultra-wide (cinematic).
  Landscape21x9,
}

impl ImageAspectRatio {
  /// Wire representation — the exact string Google expects in the
  /// `aspectRatio` field.
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Square         => "1:1",
      Self::Portrait2x3    => "2:3",
      Self::Landscape3x2   => "3:2",
      Self::Portrait3x4    => "3:4",
      Self::Landscape4x3   => "4:3",
      Self::Portrait4x5    => "4:5",
      Self::Landscape5x4   => "5:4",
      Self::Portrait9x16   => "9:16",
      Self::Landscape16x9  => "16:9",
      Self::Landscape21x9  => "21:9",
    }
  }
}

// Serialize as the wire string ("1:1", "16:9", …) rather than the default
// external-tag enum form.
impl serde::Serialize for ImageAspectRatio {
  fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(self.as_str())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const ALL: [ImageAspectRatio; 10] = [
    ImageAspectRatio::Square,
    ImageAspectRatio::Portrait2x3,
    ImageAspectRatio::Landscape3x2,
    ImageAspectRatio::Portrait3x4,
    ImageAspectRatio::Landscape4x3,
    ImageAspectRatio::Portrait4x5,
    ImageAspectRatio::Landscape5x4,
    ImageAspectRatio::Portrait9x16,
    ImageAspectRatio::Landscape16x9,
    ImageAspectRatio::Landscape21x9,
  ];

  #[test]
  fn every_variant_maps_to_a_distinct_string() {
    let mut strs: Vec<&str> = ALL.iter().map(|v| v.as_str()).collect();
    strs.sort();
    strs.dedup();
    assert_eq!(strs.len(), 10, "expected 10 distinct wire values, got {:?}", strs);
  }

  #[test]
  fn matches_docs_strings() {
    assert_eq!(ImageAspectRatio::Square.as_str(), "1:1");
    assert_eq!(ImageAspectRatio::Landscape16x9.as_str(), "16:9");
    assert_eq!(ImageAspectRatio::Portrait9x16.as_str(), "9:16");
    assert_eq!(ImageAspectRatio::Landscape21x9.as_str(), "21:9");
  }

  #[test]
  fn serializes_as_plain_wire_string() {
    assert_eq!(serde_json::to_string(&ImageAspectRatio::Square).unwrap(), "\"1:1\"");
    assert_eq!(serde_json::to_string(&ImageAspectRatio::Landscape16x9).unwrap(), "\"16:9\"");
    assert_eq!(serde_json::to_string(&ImageAspectRatio::Landscape21x9).unwrap(), "\"21:9\"");
  }
}
