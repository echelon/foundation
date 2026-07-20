/// Aspect ratios accepted by Kinovi video generation (`inputs.aspectRatio`).
///
/// The wire format is `"<width>:<height>"` — e.g. `Landscape16x9` serialises as
/// `"16:9"`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum VideoAspectRatio {
  /// 16:9 landscape (widescreen).
  Landscape16x9,
  /// 9:16 portrait (phone / vertical video).
  Portrait9x16,
  /// 1:1 square.
  Square1x1,
  /// 4:3 landscape (slight wide).
  Landscape4x3,
  /// 3:4 portrait (slight tall).
  Portrait3x4,
  /// 21:9 cinematic ultrawide.
  Cinematic21x9,
}

impl VideoAspectRatio {
  /// Wire representation — the exact string Kinovi expects in `"aspectRatio"`.
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Landscape16x9 => "16:9",
      Self::Portrait9x16  => "9:16",
      Self::Square1x1     => "1:1",
      Self::Landscape4x3  => "4:3",
      Self::Portrait3x4   => "3:4",
      Self::Cinematic21x9 => "21:9",
    }
  }
}

// Serialize as the wire string ("16:9", "9:16", …).
impl serde::Serialize for VideoAspectRatio {
  fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(self.as_str())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn every_variant_maps_to_a_distinct_string() {
    let all = [
      VideoAspectRatio::Landscape16x9,
      VideoAspectRatio::Portrait9x16,
      VideoAspectRatio::Square1x1,
      VideoAspectRatio::Landscape4x3,
      VideoAspectRatio::Portrait3x4,
      VideoAspectRatio::Cinematic21x9,
    ];
    let mut strs: Vec<&str> = all.iter().map(|v| v.as_str()).collect();
    strs.sort();
    strs.dedup();
    assert_eq!(strs.len(), 6, "expected 6 distinct wire values, got {:?}", strs);
  }

  #[test]
  fn matches_docs_strings() {
    assert_eq!(VideoAspectRatio::Landscape16x9.as_str(), "16:9");
    assert_eq!(VideoAspectRatio::Cinematic21x9.as_str(), "21:9");
  }

  #[test]
  fn serializes_as_plain_wire_string() {
    assert_eq!(serde_json::to_string(&VideoAspectRatio::Portrait9x16).unwrap(), "\"9:16\"");
    assert_eq!(serde_json::to_string(&VideoAspectRatio::Square1x1).unwrap(), "\"1:1\"");
  }
}
