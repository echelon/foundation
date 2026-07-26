use crate::api::requests::legacy_common::legacy_get;
use crate::api::types::audio_types::audio_task_result::AudioTaskResult;
use crate::api::types::legacy_types::legacy_task::LegacyTask;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::kling_client_error::KlingClientError;
use crate::error::kling_error::KlingError;

/// Which id to look up — the system `task_id`, or the caller-chosen
/// `external_task_id`. Both fill the same path slot.
#[derive(Clone, Debug)]
pub enum AudioTaskRef {
  TaskId(String),
  ExternalTaskId(String),
}

impl AudioTaskRef {
  pub(crate) fn as_path_segment(&self) -> &str {
    match self {
      Self::TaskId(id) | Self::ExternalTaskId(id) => id,
    }
  }
}

#[derive(Clone, Debug)]
pub struct GetTextToAudioTaskArgs<'a> {
  pub api_key: &'a KlingApiKey,
  pub task: AudioTaskRef,
}

/// `GET /v1/audio/text-to-audio/{id}` — poll a text-to-audio generation
/// (legacy vocabulary; status `succeed`). MP3/WAV URLs appear in
/// `task_result.audios` once terminal.
pub async fn get_text_to_audio_task(args: GetTextToAudioTaskArgs<'_>) -> Result<LegacyTask<AudioTaskResult>, KlingError> {
  let GetTextToAudioTaskArgs { api_key, task } = args;
  let id = task.as_path_segment();
  if id.trim().is_empty() {
    return Err(KlingClientError::InvalidRequest("audio task id must not be empty".into()).into());
  }
  let path = format!("/v1/audio/text-to-audio/{id}");
  legacy_get(api_key, &path, &[]).await
}

#[cfg(test)]
mod tests {
  use crate::api::types::audio_types::audio_task_result::AudioTaskResult;
  use crate::api::types::legacy_types::legacy_task::LegacyTask;
  use crate::api::types::legacy_types::legacy_task_status::LegacyTaskStatus;

  /// The documented response, with the legacy `url_mp3` field naming.
  #[test]
  fn documented_response_parses() {
    let json = r#"{
      "task_id": "738552951910035532",
      "task_status": "succeed",
      "task_result": {
        "audios": [{
          "id": "a_1",
          "url_mp3": "https://cdn.example/a.mp3",
          "url_wav": "https://cdn.example/a.wav",
          "duration_mp3": "6.5",
          "duration_wav": "6.5"
        }]
      },
      "final_unit_deduction": "0.25"
    }"#;
    let task: LegacyTask<AudioTaskResult> = serde_json::from_str(json).unwrap();
    assert_eq!(task.task_status, LegacyTaskStatus::Succeeded);
    let audios = task.task_result.unwrap().audios;
    assert_eq!(audios[0].url_mp3.as_deref(), Some("https://cdn.example/a.mp3"));
    assert_eq!(audios[0].duration_wav.as_deref(), Some("6.5"));
  }
}
