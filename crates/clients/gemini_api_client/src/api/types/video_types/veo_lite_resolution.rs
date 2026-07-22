use serde::Serialize;

/// Output resolutions for **Veo 3.1 Lite** — `parameters.resolution`. Lite has
/// no 4K tier, so it gets its own enum (an unsupported 4K request is a compile
/// error, not a runtime 400).
///
/// Docs: <https://ai.google.dev/gemini-api/docs/veo>
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum VeoLiteResolution {
  /// 720p (default).
  P720,
  /// 1080p (requires an 8-second duration).
  P1080,
}

impl VeoLiteResolution {
  /// Wire representation.
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::P720 => "720p",
      Self::P1080 => "1080p",
    }
  }
}

impl Serialize for VeoLiteResolution {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(self.as_str())
  }
}
