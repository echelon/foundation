/// The `image_generation` tool's `moderation` — content-filter strictness.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ImageModeration {
  /// Server default strictness.
  Auto,
  /// Less restrictive filtering.
  Low,
}

impl ImageModeration {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Auto => "auto",
      Self::Low => "low",
    }
  }
}

impl serde::Serialize for ImageModeration {
  fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(self.as_str())
  }
}
