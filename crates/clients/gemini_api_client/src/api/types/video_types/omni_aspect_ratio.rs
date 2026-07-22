use serde::Serialize;

/// Aspect ratios accepted by Omni's `response_format.aspect_ratio`.
///
/// Docs: <https://ai.google.dev/gemini-api/docs/omni>
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum OmniAspectRatio {
  /// 16:9 landscape (default).
  Landscape16x9,
  /// 9:16 portrait.
  Portrait9x16,
}

impl OmniAspectRatio {
  /// Wire representation.
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Landscape16x9 => "16:9",
      Self::Portrait9x16 => "9:16",
    }
  }
}

impl Serialize for OmniAspectRatio {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(self.as_str())
  }
}
