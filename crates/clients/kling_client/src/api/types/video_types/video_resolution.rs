use serde::Serialize;

/// Output resolution for video generation. Which tiers an endpoint accepts
/// varies (4K is Kling 3.0 / 3.0 Omni only); each binding validates its own
/// subset before sending.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VideoResolution {
  P720,
  P1080,
  K4,
}

impl VideoResolution {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::P720 => "720p",
      Self::P1080 => "1080p",
      Self::K4 => "4k",
    }
  }
}

impl Serialize for VideoResolution {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(self.as_str())
  }
}
