use serde::Serialize;

/// Veo's `parameters.personGeneration` people policy. Google constrains the
/// allowed value by task: text-to-video permits only [`AllowAll`](Self::AllowAll);
/// image-to-video / interpolation permit only [`AllowAdult`](Self::AllowAdult).
///
/// Docs: <https://ai.google.dev/gemini-api/docs/veo>
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum VeoPersonGeneration {
  AllowAll,
  AllowAdult,
}

impl VeoPersonGeneration {
  /// Wire representation.
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::AllowAll => "allow_all",
      Self::AllowAdult => "allow_adult",
    }
  }
}

impl Serialize for VeoPersonGeneration {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(self.as_str())
  }
}
