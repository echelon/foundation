use serde::{Deserialize, Serialize};

/// How a custom element was defined: from a reference video
/// (`video_refer` — a "video character element", realistic humanoids only,
/// supported by Kling 3.0 / 3.0 Omni video) or from reference images
/// (`image_refer` — a "multi-image element", the only kind Kling O1 video
/// accepts).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReferenceType {
  VideoRefer,
  ImageRefer,
  /// Forward-compat: a reference type this crate doesn't know yet.
  Unknown(String),
}

impl ReferenceType {
  pub fn from_wire(s: &str) -> Self {
    match s {
      "video_refer" => Self::VideoRefer,
      "image_refer" => Self::ImageRefer,
      other => Self::Unknown(other.to_string()),
    }
  }

  pub fn as_str(&self) -> &str {
    match self {
      Self::VideoRefer => "video_refer",
      Self::ImageRefer => "image_refer",
      Self::Unknown(s) => s,
    }
  }
}

impl Serialize for ReferenceType {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(self.as_str())
  }
}

impl<'de> Deserialize<'de> for ReferenceType {
  fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    let s = String::deserialize(deserializer)?;
    Ok(Self::from_wire(&s))
  }
}
