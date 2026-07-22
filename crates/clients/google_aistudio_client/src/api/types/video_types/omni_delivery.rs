use serde::Serialize;

/// How Omni returns the finished video (`response_format.delivery`).
///
/// Docs: <https://ai.google.dev/gemini-api/docs/omni>
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum OmniDelivery {
  /// Base64 inline in the response — only works for videos ≤ 4 MB.
  Inline,
  /// A Files-API handle to poll + download — required for larger outputs, and
  /// the shape our workers prefer (no giant held-open response bodies).
  Uri,
}

impl OmniDelivery {
  /// Wire representation.
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Inline => "inline",
      Self::Uri => "uri",
    }
  }
}

impl Serialize for OmniDelivery {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(self.as_str())
  }
}
