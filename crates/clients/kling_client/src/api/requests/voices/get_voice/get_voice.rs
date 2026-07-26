use crate::api::requests::legacy_common::legacy_get;
use crate::api::types::legacy_types::legacy_task::LegacyTask;
use crate::api::types::voice_types::voice_task_result::VoiceTaskResult;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::kling_client_error::KlingClientError;
use crate::error::kling_error::KlingError;

/// Which id to look up — the system `task_id` from
/// [`crate::api::requests::voices::create_voice`], or the caller-chosen
/// `external_task_id`. Both fill the same path slot.
#[derive(Clone, Debug)]
pub enum VoiceTaskRef {
  TaskId(String),
  ExternalTaskId(String),
}

impl VoiceTaskRef {
  pub(crate) fn as_path_segment(&self) -> &str {
    match self {
      Self::TaskId(id) | Self::ExternalTaskId(id) => id,
    }
  }
}

#[derive(Clone, Debug)]
pub struct GetVoiceArgs<'a> {
  pub api_key: &'a KlingApiKey,
  pub task: VoiceTaskRef,
}

/// `GET /v1/general/custom-voices/{id}` — fetch one voice creation task
/// (legacy vocabulary; status `succeed`). The created voice appears in
/// `task_result.voices` once terminal.
pub async fn get_voice(args: GetVoiceArgs<'_>) -> Result<LegacyTask<VoiceTaskResult>, KlingError> {
  let GetVoiceArgs { api_key, task } = args;
  let id = task.as_path_segment();
  if id.trim().is_empty() {
    return Err(KlingClientError::InvalidRequest("voice task id must not be empty".into()).into());
  }
  let path = format!("/v1/general/custom-voices/{id}");
  legacy_get(api_key, &path, &[]).await
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::api::types::legacy_types::legacy_task_status::LegacyTaskStatus;

  #[test]
  fn documented_response_parses() {
    let json = r#"{
      "task_id": "738552951910035532",
      "task_status": "succeed",
      "task_info": {"external_task_id": "custom_1"},
      "task_result": {
        "voices": [{"voice_id": "850087542757535834", "voice_name": "Narrator", "trial_url": "https://cdn.example/trial.mp3", "owned_by": "12345"}]
      },
      "final_unit_deduction": "0.05",
      "created_at": 1722769557708,
      "updated_at": 1722769557708
    }"#;
    let task: LegacyTask<VoiceTaskResult> = serde_json::from_str(json).unwrap();
    assert_eq!(task.task_status, LegacyTaskStatus::Succeeded);
    let voices = task.task_result.unwrap().voices;
    assert_eq!(voices[0].voice_id.as_deref(), Some("850087542757535834"));
  }
}
