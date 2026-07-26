use serde::Serialize;

/// The `audio` setting on Kling 3.0 Omni, which can also carry a reference
/// video's original sound through.
///
/// Constraints (validated per binding): `original` requires a reference
/// video; a `feature_video` requires `off`; a `base_video` forbids `native`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OmniAudio {
  /// Generate audio natively with the video.
  Native,
  /// Keep the reference video's own sound.
  Original,
  /// Silent output (the default server-side).
  Off,
}

impl OmniAudio {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Native => "native",
      Self::Original => "original",
      Self::Off => "off",
    }
  }
}

impl Serialize for OmniAudio {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(self.as_str())
  }
}
