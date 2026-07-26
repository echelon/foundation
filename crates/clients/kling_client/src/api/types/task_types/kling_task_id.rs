use serde::{Deserialize, Serialize};

/// A Kling system-generated task id (new-style endpoints) — a numeric string
/// like `"893605946402811985"`. Distinct from the caller-chosen
/// `external_task_id`, which Kling merely echoes back.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct KlingTaskId(pub String);

impl std::fmt::Display for KlingTaskId {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}
