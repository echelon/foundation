use serde::Serialize;

use crate::api::requests::legacy_common::legacy_post;
use crate::api::types::legacy_types::legacy_created_task::LegacyCreatedTask;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::kling_client_error::KlingClientError;
use crate::error::kling_error::KlingError;

pub(crate) const PATH: &str = "/v1/audio/video-to-audio";
const MAX_PROMPT_CHARS: usize = 200;

/// The source video to score — one of the two documented inputs (mutually
/// exclusive; the enum enforces exactly-one).
#[derive(Clone, Debug)]
pub enum VideoToAudioSource {
  /// A Kling-generated video id, ≤30 days old, 3.0–20.0s.
  VideoId(String),
  /// A hosted video URL: `.mp4/.mov`, ≤100MB, 3.0–20.0s.
  VideoUrl(String),
}

/// Kling video-to-audio: generate sound effects / BGM for a video. The
/// result carries both the audio files and the source video remuxed with
/// them.
#[derive(Clone, Debug)]
pub struct VideoToAudioRequest {
  pub source: VideoToAudioSource,
  /// ≤200 chars — describe the desired sound effects.
  pub sound_effect_prompt: Option<String>,
  /// ≤200 chars — describe the desired background music.
  pub bgm_prompt: Option<String>,
  /// Default false.
  pub asmr_mode: Option<bool>,
  /// Callbacks use the legacy dialect →
  /// [`crate::api::webhooks::parse_legacy_callback`].
  pub callback_url: Option<String>,
  pub external_task_id: Option<String>,
}

impl VideoToAudioRequest {
  pub fn new(source: VideoToAudioSource) -> Self {
    Self {
      source,
      sound_effect_prompt: None,
      bgm_prompt: None,
      asmr_mode: None,
      callback_url: None,
      external_task_id: None,
    }
  }
}

#[derive(Clone, Debug)]
pub struct VideoToAudioArgs<'a> {
  pub api_key: &'a KlingApiKey,
  pub request: VideoToAudioRequest,
}

#[derive(Serialize)]
pub(crate) struct VideoToAudioBody<'a> {
  #[serde(skip_serializing_if = "Option::is_none")]
  video_id: Option<&'a str>,
  #[serde(skip_serializing_if = "Option::is_none")]
  video_url: Option<&'a str>,
  #[serde(skip_serializing_if = "Option::is_none")]
  sound_effect_prompt: Option<&'a str>,
  #[serde(skip_serializing_if = "Option::is_none")]
  bgm_prompt: Option<&'a str>,
  #[serde(skip_serializing_if = "Option::is_none")]
  asmr_mode: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  callback_url: Option<&'a str>,
  #[serde(skip_serializing_if = "Option::is_none")]
  external_task_id: Option<&'a str>,
}

pub(crate) fn build_body(request: &VideoToAudioRequest) -> Result<VideoToAudioBody<'_>, KlingClientError> {
  let (video_id, video_url) = match &request.source {
    VideoToAudioSource::VideoId(id) => {
      if id.trim().is_empty() {
        return Err(KlingClientError::InvalidRequest("video_id must not be empty".into()));
      }
      (Some(id.as_str()), None)
    }
    VideoToAudioSource::VideoUrl(url) => {
      if url.trim().is_empty() {
        return Err(KlingClientError::InvalidRequest("video_url must not be empty".into()));
      }
      (None, Some(url.as_str()))
    }
  };
  for (label, prompt) in [
    ("sound_effect_prompt", &request.sound_effect_prompt),
    ("bgm_prompt", &request.bgm_prompt),
  ] {
    if let Some(prompt) = prompt {
      let chars = prompt.chars().count();
      if chars > MAX_PROMPT_CHARS {
        return Err(KlingClientError::InvalidRequest(format!(
          "{label} is {chars} chars; at most {MAX_PROMPT_CHARS} allowed"
        )));
      }
    }
  }
  Ok(VideoToAudioBody {
    video_id,
    video_url,
    sound_effect_prompt: request.sound_effect_prompt.as_deref(),
    bgm_prompt: request.bgm_prompt.as_deref(),
    asmr_mode: request.asmr_mode,
    callback_url: request.callback_url.as_deref(),
    external_task_id: request.external_task_id.as_deref(),
  })
}

/// Dry-run: the exact JSON body [`video_to_audio`] would send.
pub fn video_to_audio_wire_json(request: &VideoToAudioRequest) -> Result<serde_json::Value, KlingError> {
  let body = build_body(request)?;
  serde_json::to_value(&body).map_err(|e| KlingClientError::RequestSerializationError(e).into())
}

/// `POST /v1/audio/video-to-audio` — async; poll with
/// [`crate::api::requests::audio::get_video_to_audio_task`].
pub async fn video_to_audio(args: VideoToAudioArgs<'_>) -> Result<LegacyCreatedTask, KlingError> {
  let VideoToAudioArgs { api_key, request } = args;
  let body = build_body(&request)?;
  let body_json = serde_json::to_string(&body).map_err(KlingClientError::RequestSerializationError)?;
  legacy_post(api_key, PATH, body_json).await
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  // ── Wire-shape tests ──

  #[test]
  fn wire_shape_from_url_with_prompts() {
    let mut request = VideoToAudioRequest::new(VideoToAudioSource::VideoUrl(
      "https://example.com/clip.mp4".to_string(),
    ));
    request.sound_effect_prompt = Some("footsteps on gravel".to_string());
    request.bgm_prompt = Some("gentle piano".to_string());
    request.asmr_mode = Some(false);
    assert_eq!(
      video_to_audio_wire_json(&request).unwrap(),
      json!({
        "video_url": "https://example.com/clip.mp4",
        "sound_effect_prompt": "footsteps on gravel",
        "bgm_prompt": "gentle piano",
        "asmr_mode": false
      })
    );
  }

  #[test]
  fn wire_shape_from_video_id_never_emits_both_sources() {
    let request = VideoToAudioRequest::new(VideoToAudioSource::VideoId("893605946402811985".to_string()));
    assert_eq!(
      video_to_audio_wire_json(&request).unwrap(),
      json!({"video_id": "893605946402811985"})
    );
  }

  // ── Validation tests ──

  #[test]
  fn rejects_empty_source_and_long_prompts() {
    let empty = VideoToAudioRequest::new(VideoToAudioSource::VideoUrl(" ".to_string()));
    assert!(matches!(build_body(&empty), Err(KlingClientError::InvalidRequest(_))));

    let mut long_prompt =
      VideoToAudioRequest::new(VideoToAudioSource::VideoId("1".to_string()));
    long_prompt.bgm_prompt = Some("x".repeat(201));
    assert!(matches!(build_body(&long_prompt), Err(KlingClientError::InvalidRequest(_))));
  }
}
