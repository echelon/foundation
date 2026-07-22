use serde::Serialize;

/// Aspect ratios accepted by Veo's `parameters.aspectRatio`.
///
/// Docs: <https://ai.google.dev/gemini-api/docs/veo>
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum VeoAspectRatio {
  /// 16:9 landscape (default).
  Landscape16x9,
  /// 9:16 portrait.
  Portrait9x16,
}

impl VeoAspectRatio {
  /// Wire representation.
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Landscape16x9 => "16:9",
      Self::Portrait9x16 => "9:16",
    }
  }
}

impl Serialize for VeoAspectRatio {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(self.as_str())
  }
}
