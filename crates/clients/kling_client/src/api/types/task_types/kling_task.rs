use serde::Deserialize;

use crate::api::types::task_types::billing_entry::BillingEntry;
use crate::api::types::task_types::kling_task_id::KlingTaskId;
use crate::api::types::task_types::kling_task_status::KlingTaskStatus;
use crate::api::types::task_types::task_output::TaskOutput;

/// A full task record from the new-style query endpoints (`GET/POST /tasks`)
/// — also the exact payload Kling POSTs to `callback_url` on status changes.
///
/// A failed task is an `Ok(KlingTask)` with `status == Failed` and the reason
/// in `message` — only transport/HTTP problems are `Err` at the binding layer.
#[derive(Clone, Debug, Deserialize)]
pub struct KlingTask {
  pub id: KlingTaskId,
  pub status: KlingTaskStatus,
  /// Failure reason when `status == Failed` (e.g. content risk control).
  #[serde(default)]
  pub message: Option<String>,
  /// Unix timestamps in milliseconds.
  #[serde(default)]
  pub create_time: Option<i64>,
  #[serde(default)]
  pub update_time: Option<i64>,
  /// The caller-chosen `external_task_id`, echoed back (if one was set).
  #[serde(default)]
  pub external_id: Option<String>,
  #[serde(default)]
  pub outputs: Vec<TaskOutput>,
  #[serde(default)]
  pub billing: Vec<BillingEntry>,
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::api::types::task_types::task_output::TaskOutput;

  #[test]
  fn documented_query_response_task_parses() {
    let json = r#"{
      "id": "893605946402811985",
      "status": "succeeded",
      "message": "",
      "create_time": 1781080778802,
      "update_time": 1781080794151,
      "external_id": "job_01ABC",
      "outputs": [
        {"type": "video", "id": "v_1", "url": "https://cdn.example/v.mp4", "watermark_url": "https://cdn.example/vw.mp4", "duration": "5.0"}
      ],
      "billing": [
        {"charge_type": "unit", "amount": "4.0", "package_type": "video"}
      ]
    }"#;
    let task: KlingTask = serde_json::from_str(json).unwrap();
    assert_eq!(task.id.0, "893605946402811985");
    assert!(task.status.is_terminal());
    assert_eq!(task.external_id.as_deref(), Some("job_01ABC"));
    assert!(matches!(task.outputs[0], TaskOutput::Video(_)));
    assert_eq!(task.billing[0].charge_type.as_deref(), Some("unit"));
  }

  #[test]
  fn minimal_processing_task_parses() {
    let json = r#"{"id": "1", "status": "processing"}"#;
    let task: KlingTask = serde_json::from_str(json).unwrap();
    assert!(!task.status.is_terminal());
    assert!(task.outputs.is_empty());
    assert!(task.billing.is_empty());
  }
}
