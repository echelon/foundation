use serde::Deserialize;

use crate::api::types::element_types::element_record::ElementRecord;
use crate::api::types::legacy_types::legacy_audio_result::LegacyAudioResult;
use crate::api::types::legacy_types::legacy_balance_deduction::LegacyBalanceDeduction;
use crate::api::types::legacy_types::legacy_image_result::LegacyImageResult;
use crate::api::types::legacy_types::legacy_task_info::LegacyTaskInfo;
use crate::api::types::legacy_types::legacy_task_status::LegacyTaskStatus;
use crate::api::types::legacy_types::legacy_video_result::LegacyVideoResult;
use crate::api::types::voice_types::voice_record::VoiceRecord;

/// A legacy-style callback payload (image, audio, element and voice
/// management tasks): `task_id`/`task_status` vocabulary, statuses spelled
/// `succeed`.
#[derive(Clone, Debug, Deserialize)]
pub struct LegacyCallback {
  pub task_id: String,
  pub task_status: LegacyTaskStatus,
  #[serde(default)]
  pub task_status_msg: Option<String>,
  /// Unix timestamps in milliseconds.
  #[serde(default)]
  pub created_at: Option<i64>,
  #[serde(default)]
  pub updated_at: Option<i64>,
  /// Units deducted from a resource package — a decimal string.
  #[serde(default)]
  pub final_unit_deduction: Option<String>,
  #[serde(default)]
  pub final_balance_deduction: Option<LegacyBalanceDeduction>,
  #[serde(default)]
  pub task_info: Option<LegacyTaskInfo>,
  #[serde(default)]
  pub task_result: Option<LegacyCallbackResult>,
}

/// The result payload of a legacy callback — which array is populated depends
/// on the task family.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct LegacyCallbackResult {
  #[serde(default)]
  pub images: Vec<LegacyImageResult>,
  #[serde(default)]
  pub videos: Vec<LegacyVideoResult>,
  #[serde(default)]
  pub audios: Vec<LegacyAudioResult>,
  #[serde(default)]
  pub elements: Vec<ElementRecord>,
  #[serde(default)]
  pub voices: Vec<VoiceRecord>,
}

/// Parse a legacy-style callback body.
///
/// Kling callbacks are **unsigned**: never trust the payload alone. Match it
/// to a job by `task_info.external_task_id`, then confirm by re-querying the
/// task before acting.
pub fn parse_legacy_callback(body: &str) -> Result<LegacyCallback, serde_json::Error> {
  serde_json::from_str(body)
}

#[cfg(test)]
mod tests {
  use super::*;

  /// The documented legacy callback example (abridged).
  #[test]
  fn documented_legacy_callback_parses() {
    let body = r#"{
      "task_id": "738552951910035532",
      "task_status": "succeed",
      "task_status_msg": "",
      "created_at": 1722769557708,
      "updated_at": 1722769557708,
      "final_unit_deduction": "8",
      "final_balance_deduction": {"quota": "0.028", "list_price": "0.028"},
      "task_info": {
        "parent_video": {"id": "pv_1", "url": "https://cdn.example/parent.mp4", "duration": "10"},
        "external_task_id": "job_01ABC"
      },
      "task_result": {
        "images": [{"index": 0, "url": "https://cdn.example/i.png"}],
        "videos": [{"id": "v_1", "url": "https://cdn.example/v.mp4", "duration": "5"}]
      }
    }"#;
    let callback = parse_legacy_callback(body).unwrap();
    assert_eq!(callback.task_status, LegacyTaskStatus::Succeeded);
    assert_eq!(
      callback.task_info.as_ref().unwrap().external_task_id.as_deref(),
      Some("job_01ABC")
    );
    let result = callback.task_result.unwrap();
    assert_eq!(result.images.len(), 1);
    assert_eq!(result.videos.len(), 1);
    assert!(result.elements.is_empty());
  }

  #[test]
  fn element_creation_callback_parses() {
    let body = r#"{
      "task_id": "1",
      "task_status": "succeed",
      "task_result": {
        "elements": [{"element_id": 829836802793406551, "element_name": "Zhang", "reference_type": "image_refer", "status": "succeed"}]
      }
    }"#;
    let callback = parse_legacy_callback(body).unwrap();
    let elements = &callback.task_result.unwrap().elements;
    assert_eq!(elements[0].element_id.as_string(), "829836802793406551");
  }

  #[test]
  fn garbage_body_is_an_error_not_a_panic() {
    assert!(parse_legacy_callback("[]").is_err());
  }
}
