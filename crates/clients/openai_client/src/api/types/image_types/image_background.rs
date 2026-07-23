/// The `image_generation` tool's `background` — output transparency.
/// NOTE: gpt-image-2 does NOT support `Transparent` (rejected client-side by
/// that binding before any HTTP).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ImageBackground {
  /// Server default.
  Auto,
  Opaque,
  /// Transparent background (png/webp outputs) — not supported by gpt-image-2.
  Transparent,
}

impl ImageBackground {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Auto => "auto",
      Self::Opaque => "opaque",
      Self::Transparent => "transparent",
    }
  }
}

impl serde::Serialize for ImageBackground {
  fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(self.as_str())
  }
}
