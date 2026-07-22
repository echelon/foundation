use serde::Serialize;

/// Output resolutions for **Veo 3.1** and **Veo 3.1 Fast** —
/// `parameters.resolution`. Resolution picks the price tier. Veo 3.1 Lite has
/// no 4K tier and uses [`super::veo_lite_resolution::VeoLiteResolution`], so an
/// unsupported combination is unrepresentable.
///
/// Docs: <https://ai.google.dev/gemini-api/docs/veo>
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum VeoResolution {
  /// 720p (default).
  P720,
  /// 1080p (requires an 8-second duration).
  P1080,
  /// 4K (requires an 8-second duration; the expensive tier).
  K4,
}

impl VeoResolution {
  /// Wire representation.
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::P720 => "720p",
      Self::P1080 => "1080p",
      Self::K4 => "4k",
    }
  }
}

impl Serialize for VeoResolution {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(self.as_str())
  }
}
