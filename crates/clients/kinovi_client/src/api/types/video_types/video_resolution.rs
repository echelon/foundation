/// Output resolution for Kinovi video generation (`inputs.outputResolution`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum VideoResolution {
  /// `"480p"` — standard definition.
  P480,
  /// `"720p"` — HD.
  P720,
  /// `"1080p"` — full HD.
  P1080,
  /// `"4k"` — ultra HD.
  K4,
}

impl VideoResolution {
  /// Wire representation — the exact string Kinovi expects in `"outputResolution"`.
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::P480  => "480p",
      Self::P720  => "720p",
      Self::P1080 => "1080p",
      Self::K4    => "4k",
    }
  }
}

// Serialize as the wire string ("480p", "720p", "1080p", "4k").
impl serde::Serialize for VideoResolution {
  fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(self.as_str())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn matches_docs_strings() {
    assert_eq!(VideoResolution::P480.as_str(), "480p");
    assert_eq!(VideoResolution::P720.as_str(), "720p");
    assert_eq!(VideoResolution::P1080.as_str(), "1080p");
    assert_eq!(VideoResolution::K4.as_str(), "4k");
  }

  #[test]
  fn serializes_as_plain_wire_string() {
    assert_eq!(serde_json::to_string(&VideoResolution::P720).unwrap(), "\"720p\"");
    assert_eq!(serde_json::to_string(&VideoResolution::K4).unwrap(), "\"4k\"");
  }
}
