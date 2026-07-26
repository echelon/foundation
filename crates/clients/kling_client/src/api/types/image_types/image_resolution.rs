use serde::Serialize;

/// Output resolution for image generation. 4K is Kling Image 3.0 Omni only;
/// each binding validates its own subset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageResolution {
  R1k,
  R2k,
  R4k,
}

impl ImageResolution {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::R1k => "1k",
      Self::R2k => "2k",
      Self::R4k => "4k",
    }
  }
}

impl Serialize for ImageResolution {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(self.as_str())
  }
}
