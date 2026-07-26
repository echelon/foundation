use serde::Serialize;

/// Aspect ratio for image generation. `Auto` (follow the reference inputs)
/// exists only on the omni-image endpoint; the plain generations endpoint
/// rejects it (validated per binding).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageAspectRatio {
  Landscape16x9,
  Portrait9x16,
  Square1x1,
  Landscape4x3,
  Portrait3x4,
  Landscape3x2,
  Portrait2x3,
  Cinema21x9,
  Auto,
}

impl ImageAspectRatio {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Landscape16x9 => "16:9",
      Self::Portrait9x16 => "9:16",
      Self::Square1x1 => "1:1",
      Self::Landscape4x3 => "4:3",
      Self::Portrait3x4 => "3:4",
      Self::Landscape3x2 => "3:2",
      Self::Portrait2x3 => "2:3",
      Self::Cinema21x9 => "21:9",
      Self::Auto => "auto",
    }
  }
}

impl Serialize for ImageAspectRatio {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(self.as_str())
  }
}
