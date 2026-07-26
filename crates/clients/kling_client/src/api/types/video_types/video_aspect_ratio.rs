use serde::Serialize;

/// Frame aspect ratio for text-to-video (image-to-video follows the input
/// image instead and has no aspect setting).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VideoAspectRatio {
  Landscape16x9,
  Portrait9x16,
  Square1x1,
}

impl VideoAspectRatio {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Landscape16x9 => "16:9",
      Self::Portrait9x16 => "9:16",
      Self::Square1x1 => "1:1",
    }
  }
}

impl Serialize for VideoAspectRatio {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(self.as_str())
  }
}
