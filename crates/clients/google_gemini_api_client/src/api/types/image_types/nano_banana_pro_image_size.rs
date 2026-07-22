use serde::Serialize;

/// Output size tiers for **Nano Banana Pro** (`gemini-3-pro-image`) —
/// `generationConfig.imageConfig.imageSize`. 1K and 2K share a price tier; 4K
/// is its own. No 512px tier on Pro.
///
/// Docs: <https://ai.google.dev/gemini-api/docs/image-generation>
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NanoBananaProImageSize {
  /// 1K (default).
  K1,
  /// 2K.
  K2,
  /// 4K.
  K4,
}

impl NanoBananaProImageSize {
  /// Wire representation (uppercase required by the API).
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::K1 => "1K",
      Self::K2 => "2K",
      Self::K4 => "4K",
    }
  }
}

impl Serialize for NanoBananaProImageSize {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(self.as_str())
  }
}
