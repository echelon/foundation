use serde::Serialize;

/// The `audio` setting where the choice is generated ("native") audio or
/// silence — Kling 3.0 and 2.6. (Kling 3.0 Omni adds `original`, see
/// [`super::omni_audio::OmniAudio`]; motion control and O1 offer
/// `original`/`off`, see [`super::source_audio::SourceAudio`].)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeAudio {
  /// Generate audio natively with the video.
  Native,
  /// Silent output (the default server-side).
  Off,
}

impl NativeAudio {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Native => "native",
      Self::Off => "off",
    }
  }
}

impl Serialize for NativeAudio {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(self.as_str())
  }
}
