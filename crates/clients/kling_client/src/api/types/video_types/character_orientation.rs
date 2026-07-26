use serde::Serialize;

/// Motion control's required `character_orientation`: whether the output
/// character's facing follows the appearance image or the motion reference
/// video. With an element as the appearance source, orientation always
/// follows the video.
///
/// Also caps the motion video length: ≤30s when `Video`, ≤10s when `Image`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharacterOrientation {
  Image,
  Video,
}

impl CharacterOrientation {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Image => "image",
      Self::Video => "video",
    }
  }
}

impl Serialize for CharacterOrientation {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(self.as_str())
  }
}
