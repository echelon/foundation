/// Models accepted by the Gemini image endpoint
/// (`POST /v1beta/models/{model}:generateContent`) — the **Nano Banana** family.
///
/// Google may add or deprecate models faster than this crate ships releases, so
/// the [`GeminiImageModel::Custom`] escape hatch lets callers pass an arbitrary
/// identifier without waiting for a code change.
///
/// Docs: <https://ai.google.dev/gemini-api/docs/image-generation>
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GeminiImageModel {
  /// `gemini-2.5-flash-image` — **Nano Banana**. The standard, low-cost image
  /// model (~$0.02 / image).
  NanoBanana,

  /// `gemini-3-pro-image` — **Nano Banana Pro**. The highest-fidelity image
  /// model.
  NanoBananaPro,

  /// `gemini-3.1-flash-image` — **Nano Banana 2**. The next-generation flash
  /// image model.
  NanoBanana2,

  /// Escape hatch for model identifiers not yet enumerated here (forward compat
  /// and legacy/deprecated identifiers that still function until Google removes
  /// them).
  Custom(String),
}

impl GeminiImageModel {
  /// Wire representation — the exact string Google expects in the `{model}`
  /// path segment.
  pub fn as_str(&self) -> &str {
    match self {
      Self::NanoBanana    => "gemini-2.5-flash-image",
      Self::NanoBananaPro => "gemini-3-pro-image",
      Self::NanoBanana2   => "gemini-3.1-flash-image",
      Self::Custom(s)     => s.as_str(),
    }
  }
}

// Serialize as the wire string rather than the default external-tag enum
// format. Lets `GenerateImageRequest` round-trip through any log/audit pipeline
// as readable JSON.
impl serde::Serialize for GeminiImageModel {
  fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(self.as_str())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn nano_banana_serializes() {
    assert_eq!(GeminiImageModel::NanoBanana.as_str(), "gemini-2.5-flash-image");
  }

  #[test]
  fn nano_banana_pro_serializes() {
    assert_eq!(GeminiImageModel::NanoBananaPro.as_str(), "gemini-3-pro-image");
  }

  #[test]
  fn nano_banana_2_serializes() {
    assert_eq!(GeminiImageModel::NanoBanana2.as_str(), "gemini-3.1-flash-image");
  }

  #[test]
  fn custom_model_passes_through() {
    let m = GeminiImageModel::Custom("gemini-future-image".to_string());
    assert_eq!(m.as_str(), "gemini-future-image");
  }

  #[test]
  fn all_named_variants_are_distinct_strings() {
    let mut strs = [
      GeminiImageModel::NanoBanana.as_str(),
      GeminiImageModel::NanoBananaPro.as_str(),
      GeminiImageModel::NanoBanana2.as_str(),
    ];
    strs.sort();
    let len_before = strs.len();
    let mut deduped = strs.to_vec();
    deduped.dedup();
    assert_eq!(deduped.len(), len_before);
  }

  #[test]
  fn serializes_as_plain_wire_string() {
    assert_eq!(serde_json::to_string(&GeminiImageModel::NanoBanana).unwrap(), "\"gemini-2.5-flash-image\"");
    assert_eq!(serde_json::to_string(&GeminiImageModel::NanoBananaPro).unwrap(), "\"gemini-3-pro-image\"");
    assert_eq!(serde_json::to_string(&GeminiImageModel::NanoBanana2).unwrap(), "\"gemini-3.1-flash-image\"");
    let m = GeminiImageModel::Custom("gemini-legacy-image".to_string());
    assert_eq!(serde_json::to_string(&m).unwrap(), "\"gemini-legacy-image\"");
  }
}
