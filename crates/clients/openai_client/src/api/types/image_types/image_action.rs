/// The `image_generation` tool's `action` — what kind of image work the tool
/// performs. `Auto` (the server default) infers from the input: bare prompt →
/// generate, prompt + input images → edit.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ImageAction {
  Auto,
  Generate,
  Edit,
}

impl ImageAction {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Auto => "auto",
      Self::Generate => "generate",
      Self::Edit => "edit",
    }
  }
}

impl serde::Serialize for ImageAction {
  fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(self.as_str())
  }
}
