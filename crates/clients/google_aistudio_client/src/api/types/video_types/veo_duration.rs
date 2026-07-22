use serde::Serialize;

/// Clip lengths accepted by Veo's `parameters.durationSeconds`. 1080p, 4K, and
/// reference-image requests require [`S8`](Self::S8).
///
/// Docs: <https://ai.google.dev/gemini-api/docs/veo>
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum VeoDuration {
  /// 4 seconds.
  S4,
  /// 6 seconds.
  S6,
  /// 8 seconds (default; required for 1080p/4K).
  S8,
}

impl VeoDuration {
  /// Wire representation AND billing quantity — the API takes `durationSeconds`
  /// as a NUMBER (the docs show a string; the live API rejects strings).
  pub fn seconds(&self) -> i64 {
    match self {
      Self::S4 => 4,
      Self::S6 => 6,
      Self::S8 => 8,
    }
  }
}

impl Serialize for VeoDuration {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_i64(self.seconds())
  }
}
