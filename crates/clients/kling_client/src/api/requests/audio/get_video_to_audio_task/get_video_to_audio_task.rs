use crate::api::requests::audio::get_text_to_audio_task::get_text_to_audio_task::AudioTaskRef;
use crate::api::requests::legacy_common::legacy_get;
use crate::api::types::audio_types::audio_task_result::AudioTaskResult;
use crate::api::types::legacy_types::legacy_task::LegacyTask;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::kling_client_error::KlingClientError;
use crate::error::kling_error::KlingError;

#[derive(Clone, Debug)]
pub struct GetVideoToAudioTaskArgs<'a> {
  pub api_key: &'a KlingApiKey,
  pub task: AudioTaskRef,
}

/// `GET /v1/audio/video-to-audio/{id}` — poll a video-to-audio generation
/// (legacy vocabulary; status `succeed`). The generated sound lands in
/// `task_result.audios`, the remuxed video in `task_result.videos`, and the
/// source video in `task_info.parent_video`.
pub async fn get_video_to_audio_task(
  args: GetVideoToAudioTaskArgs<'_>,
) -> Result<LegacyTask<AudioTaskResult>, KlingError> {
  let GetVideoToAudioTaskArgs { api_key, task } = args;
  let id = task.as_path_segment();
  if id.trim().is_empty() {
    return Err(KlingClientError::InvalidRequest("audio task id must not be empty".into()).into());
  }
  let path = format!("/v1/audio/video-to-audio/{id}");
  legacy_get(api_key, &path, &[]).await
}

#[cfg(test)]
mod tests {
  use crate::api::types::audio_types::audio_task_result::AudioTaskResult;
  use crate::api::types::legacy_types::legacy_task::LegacyTask;

  /// Video-to-audio results carry both the audio and the remuxed video, plus
  /// the source as `parent_video`.
  #[test]
  fn documented_response_parses() {
    let json = r#"{
      "task_id": "1",
      "task_status": "succeed",
      "task_info": {
        "external_task_id": "job_01ABC",
        "parent_video": {"id": "pv_1", "url": "https://cdn.example/src.mp4", "duration": "10"}
      },
      "task_result": {
        "videos": [{"id": "v_1", "url": "https://cdn.example/muxed.mp4", "duration": "10"}],
        "audios": [{"id": "a_1", "url_mp3": "https://cdn.example/a.mp3", "url_wav": "https://cdn.example/a.wav"}]
      }
    }"#;
    let task: LegacyTask<AudioTaskResult> = serde_json::from_str(json).unwrap();
    let info = task.task_info.as_ref().unwrap();
    assert_eq!(info.parent_video.as_ref().unwrap().url, "https://cdn.example/src.mp4");
    let result = task.task_result.unwrap();
    assert_eq!(result.videos.len(), 1);
    assert_eq!(result.audios.len(), 1);
  }
}
