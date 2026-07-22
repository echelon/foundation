use serde::Serialize;

/// Output size tiers for **Nano Banana 2** (`gemini-3.1-flash-image`) —
/// `generationConfig.imageConfig.imageSize`. Size picks the price tier, so this
/// enum is per-model (Pro has no 512px tier; Lite is 1K-only and takes no size).
///
/// Docs: <https://ai.google.dev/gemini-api/docs/image-generation>
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NanoBanana2ImageSize {
  /// 512px (0.5K) — the cheap draft tier, Flash only.
  P512,
  /// 1K (default).
  K1,
  /// 2K.
  K2,
  /// 4K.
  K4,
}

impl NanoBanana2ImageSize {
  /// Wire representation (uppercase required by the API).
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::P512 => "512px",
      Self::K1 => "1K",
      Self::K2 => "2K",
      Self::K4 => "4K",
    }
  }
}

impl Serialize for NanoBanana2ImageSize {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(self.as_str())
  }
}
