use serde::Serialize;

/// The `audio` setting where the only choice is keeping the source video's
/// sound or silence — motion control (3.0 / 2.6) and Kling O1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceAudio {
  /// Keep the reference/source video's own sound (motion control default).
  Original,
  /// Silent output.
  Off,
}

impl SourceAudio {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Original => "original",
      Self::Off => "off",
    }
  }
}

impl Serialize for SourceAudio {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(self.as_str())
  }
}
