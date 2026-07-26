use serde::{Deserialize, Serialize};

/// Task status on legacy-style `/v1/*` endpoints (images, audio, element and
/// voice management).
///
/// Kling's legacy vocabulary spells success **`succeed`** (no "-ed") — a
/// different enum from the new-style
/// [`crate::api::types::task_types::kling_task_status::KlingTaskStatus`] on
/// purpose, so the two vocabularies can't be mixed up. `from_wire` also
/// accepts `succeeded` defensively.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LegacyTaskStatus {
  Submitted,
  Processing,
  Succeeded,
  Failed,
  /// Forward-compat: a status string this crate doesn't know yet.
  Unknown(String),
}

impl LegacyTaskStatus {
  pub fn from_wire(s: &str) -> Self {
    match s {
      "submitted" => Self::Submitted,
      "processing" => Self::Processing,
      "succeed" | "succeeded" => Self::Succeeded,
      "failed" => Self::Failed,
      other => Self::Unknown(other.to_string()),
    }
  }

  /// The wire spelling — note `succeed`.
  pub fn as_str(&self) -> &str {
    match self {
      Self::Submitted => "submitted",
      Self::Processing => "processing",
      Self::Succeeded => "succeed",
      Self::Failed => "failed",
      Self::Unknown(s) => s,
    }
  }

  /// `Unknown` is deliberately terminal: if Kling grows a new status we'd
  /// rather stop polling and surface it than spin forever.
  pub fn is_terminal(&self) -> bool {
    !matches!(self, Self::Submitted | Self::Processing)
  }
}

impl Serialize for LegacyTaskStatus {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(self.as_str())
  }
}

impl<'de> Deserialize<'de> for LegacyTaskStatus {
  fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    let s = String::deserialize(deserializer)?;
    Ok(Self::from_wire(&s))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn succeed_spelling_parses_and_round_trips() {
    let status = LegacyTaskStatus::from_wire("succeed");
    assert_eq!(status, LegacyTaskStatus::Succeeded);
    assert_eq!(status.as_str(), "succeed");
  }

  #[test]
  fn succeeded_spelling_is_accepted_defensively() {
    assert_eq!(LegacyTaskStatus::from_wire("succeeded"), LegacyTaskStatus::Succeeded);
  }

  #[test]
  fn terminality() {
    assert!(!LegacyTaskStatus::Submitted.is_terminal());
    assert!(!LegacyTaskStatus::Processing.is_terminal());
    assert!(LegacyTaskStatus::Succeeded.is_terminal());
    assert!(LegacyTaskStatus::Failed.is_terminal());
    assert!(LegacyTaskStatus::Unknown("queued".to_string()).is_terminal());
  }
}
