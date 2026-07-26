use serde::Deserialize;

use crate::api::types::legacy_types::legacy_task_info::LegacyTaskInfo;
use crate::api::types::legacy_types::legacy_task_status::LegacyTaskStatus;

/// The `data` payload of a legacy-style create response: the `task_id` to
/// poll with plus the initial status (normally `submitted`).
#[derive(Clone, Debug, Deserialize)]
pub struct LegacyCreatedTask {
  pub task_id: String,
  pub task_status: LegacyTaskStatus,
  #[serde(default)]
  pub task_info: Option<LegacyTaskInfo>,
  /// Unix timestamps in milliseconds.
  #[serde(default)]
  pub created_at: Option<i64>,
  #[serde(default)]
  pub updated_at: Option<i64>,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn documented_create_response_parses() {
    let json = r#"{
      "task_id": "738552951910035532",
      "task_status": "submitted",
      "task_info": {"external_task_id": "custom_1"},
      "created_at": 1722769557708,
      "updated_at": 1722769557708
    }"#;
    let created: LegacyCreatedTask = serde_json::from_str(json).unwrap();
    assert_eq!(created.task_id, "738552951910035532");
    assert_eq!(created.task_status, LegacyTaskStatus::Submitted);
    assert_eq!(
      created.task_info.unwrap().external_task_id.as_deref(),
      Some("custom_1")
    );
  }
}
