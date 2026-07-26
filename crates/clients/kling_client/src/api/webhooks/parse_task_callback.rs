use crate::api::types::task_types::kling_task::KlingTask;

/// Parse a new-style callback body — the payload is a full
/// [`KlingTask`], exactly as `GET /tasks` would return it (the one wrinkle,
/// element `materials` vs `references`, is absorbed by a serde alias).
///
/// Kling callbacks are **unsigned**: never trust the payload alone. Match it
/// to a job by `external_id`, then confirm by polling
/// [`crate::api::requests::tasks::get_tasks`] before acting.
pub fn parse_task_callback(body: &str) -> Result<KlingTask, serde_json::Error> {
  serde_json::from_str(body)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::api::types::task_types::kling_task_status::KlingTaskStatus;
  use crate::api::types::task_types::task_output::TaskOutput;

  /// The documented callback example (abridged): a succeeded video task with
  /// an element output whose materials array uses the callback spelling.
  #[test]
  fn documented_callback_body_parses() {
    let body = r#"{
      "id": "893605946402811985",
      "status": "succeeded",
      "message": "",
      "create_time": 1722769557708,
      "update_time": 1722769557708,
      "external_id": "job_01ABC",
      "outputs": [
        {"type": "video", "id": "v_1", "url": "https://cdn.example/v.mp4", "watermark_url": "https://cdn.example/vw.mp4", "duration": "10.0"},
        {"type": "element", "id": "e_1", "name": "Zhang", "element_type": "video_character_elements",
         "materials": [{"type": "image", "role": "refer", "url": "https://cdn.example/r.png"}],
         "owned_by": "12345", "status": "succeeded"}
      ],
      "billing": [
        {"charge_type": "cash", "amount": "1.12", "currency": "USD", "list_price": "1.40"}
      ]
    }"#;
    let task = parse_task_callback(body).unwrap();
    assert_eq!(task.status, KlingTaskStatus::Succeeded);
    assert_eq!(task.external_id.as_deref(), Some("job_01ABC"));
    assert_eq!(task.outputs.len(), 2);
    match &task.outputs[1] {
      TaskOutput::Element(e) => assert_eq!(e.references.len(), 1),
      other => panic!("expected element, got: {other:?}"),
    }
    assert_eq!(task.billing[0].currency.as_deref(), Some("USD"));
  }

  #[test]
  fn processing_callback_parses_without_outputs() {
    let body = r#"{"id": "1", "status": "processing", "create_time": 1722769557708, "update_time": 1722769557708}"#;
    let task = parse_task_callback(body).unwrap();
    assert!(!task.status.is_terminal());
  }

  #[test]
  fn garbage_body_is_an_error_not_a_panic() {
    assert!(parse_task_callback("not json").is_err());
  }
}
