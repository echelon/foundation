use serde::{Deserialize, Serialize};

/// Task status on new-style (model-version-in-path) endpoints.
///
/// Note the spelling: the new API says `succeeded`; the legacy `/v1/*`
/// endpoints say `succeed` and are covered by
/// [`crate::api::types::legacy_types::legacy_task_status::LegacyTaskStatus`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KlingTaskStatus {
  Submitted,
  Processing,
  Succeeded,
  Failed,
  /// Forward-compat: a status string this crate doesn't know yet.
  Unknown(String),
}

impl KlingTaskStatus {
  pub fn from_wire(s: &str) -> Self {
    match s {
      "submitted" => Self::Submitted,
      "processing" => Self::Processing,
      "succeeded" => Self::Succeeded,
      "failed" => Self::Failed,
      other => Self::Unknown(other.to_string()),
    }
  }

  pub fn as_str(&self) -> &str {
    match self {
      Self::Submitted => "submitted",
      Self::Processing => "processing",
      Self::Succeeded => "succeeded",
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

impl Serialize for KlingTaskStatus {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(self.as_str())
  }
}

impl<'de> Deserialize<'de> for KlingTaskStatus {
  fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    let s = String::deserialize(deserializer)?;
    Ok(Self::from_wire(&s))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn wire_round_trip() {
    for s in ["submitted", "processing", "succeeded", "failed"] {
      assert_eq!(KlingTaskStatus::from_wire(s).as_str(), s);
    }
  }

  #[test]
  fn terminality() {
    assert!(!KlingTaskStatus::Submitted.is_terminal());
    assert!(!KlingTaskStatus::Processing.is_terminal());
    assert!(KlingTaskStatus::Succeeded.is_terminal());
    assert!(KlingTaskStatus::Failed.is_terminal());
    assert!(KlingTaskStatus::Unknown("paused".to_string()).is_terminal());
  }

  #[test]
  fn legacy_spelling_is_not_silently_accepted() {
    // "succeed" belongs to the legacy vocabulary; here it must surface as
    // Unknown (terminal) rather than masquerade as Succeeded.
    assert_eq!(
      KlingTaskStatus::from_wire("succeed"),
      KlingTaskStatus::Unknown("succeed".to_string())
    );
  }
}
