use serde::Deserialize;

use crate::api::types::task_types::kling_task_id::KlingTaskId;
use crate::api::types::task_types::kling_task_status::KlingTaskStatus;

/// The `data` payload of every new-style create-task response: the handle to
/// poll with (`id`) plus the initial status (normally `submitted`).
#[derive(Clone, Debug, Deserialize)]
pub struct CreatedTask {
  pub id: KlingTaskId,
  pub status: KlingTaskStatus,
  /// Unix timestamps in milliseconds.
  #[serde(default)]
  pub create_time: Option<i64>,
  #[serde(default)]
  pub update_time: Option<i64>,
  /// The caller-chosen `external_task_id`, echoed back (if one was set).
  #[serde(default)]
  pub external_id: Option<String>,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn documented_create_response_parses() {
    let json = r#"{
      "id": "893605946402811985",
      "status": "submitted",
      "create_time": 1781080778802,
      "update_time": 1781080794151,
      "external_id": "job_01ABC"
    }"#;
    let created: CreatedTask = serde_json::from_str(json).unwrap();
    assert_eq!(created.id.0, "893605946402811985");
    assert_eq!(created.status, KlingTaskStatus::Submitted);
    assert_eq!(created.external_id.as_deref(), Some("job_01ABC"));
  }
}
