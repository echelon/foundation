use crate::api::requests::video::common::{
  create_video_task, options_wire, validate_prompt, CreateVideoTaskParts, SettingsWire, VideoCreateBody,
};
use crate::api::types::task_types::created_task::CreatedTask;
use crate::api::types::task_types::request_options::RequestOptions;
use crate::api::types::video_types::clip_duration::ClipDuration;
use crate::api::types::video_types::video_aspect_ratio::VideoAspectRatio;
use crate::api::types::video_types::video_resolution::VideoResolution;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::kling_client_error::KlingClientError;
use crate::error::kling_error::KlingError;

pub(crate) const PATH: &str = "/text-to-video/kling-2.5-turbo";
const MAX_PROMPT_CHARS: usize = 2500;

/// Kling 2.5 Turbo text-to-video: 5s/10s clips, no audio settings.
#[derive(Clone, Debug)]
pub struct TextToVideoKling25TurboRequest {
  /// ≤2500 chars.
  pub prompt: String,
  /// `720p` (default) or `1080p`.
  pub resolution: Option<VideoResolution>,
  /// Default 16:9.
  pub aspect_ratio: Option<VideoAspectRatio>,
  /// 5s (default) or 10s.
  pub duration: Option<ClipDuration>,
  pub options: RequestOptions,
}

impl TextToVideoKling25TurboRequest {
  pub fn new(prompt: impl Into<String>) -> Self {
    Self {
      prompt: prompt.into(),
      resolution: None,
      aspect_ratio: None,
      duration: None,
      options: RequestOptions::default(),
    }
  }
}

#[derive(Clone, Debug)]
pub struct TextToVideoKling25TurboArgs<'a> {
  pub api_key: &'a KlingApiKey,
  pub request: TextToVideoKling25TurboRequest,
}

pub(crate) fn build_body(request: &TextToVideoKling25TurboRequest) -> Result<VideoCreateBody<'_>, KlingClientError> {
  validate_prompt(&request.prompt, MAX_PROMPT_CHARS)?;
  if request.resolution == Some(VideoResolution::K4) {
    return Err(KlingClientError::InvalidRequest("kling-2.5-turbo supports 720p/1080p only, not 4k".into()));
  }
  Ok(VideoCreateBody {
    prompt: Some(&request.prompt),
    contents: vec![],
    settings: SettingsWire {
      resolution: request.resolution.map(|r| r.as_str()),
      aspect_ratio: request.aspect_ratio.map(|a| a.as_str()),
      duration: request.duration.map(|d| d.seconds()),
      ..Default::default()
    }
    .into_option(),
    options: options_wire(&request.options),
  })
}

/// Dry-run: the exact JSON body [`text_to_video_kling_2_5_turbo`] would send.
pub fn text_to_video_kling_2_5_turbo_wire_json(
  request: &TextToVideoKling25TurboRequest,
) -> Result<serde_json::Value, KlingError> {
  let body = build_body(request)?;
  serde_json::to_value(&body).map_err(|e| KlingClientError::RequestSerializationError(e).into())
}

/// `POST /text-to-video/kling-2.5-turbo` — async; poll the returned task id
/// via [`crate::api::requests::tasks`].
pub async fn text_to_video_kling_2_5_turbo(args: TextToVideoKling25TurboArgs<'_>) -> Result<CreatedTask, KlingError> {
  let TextToVideoKling25TurboArgs { api_key, request } = args;
  let body = build_body(&request)?;
  create_video_task(CreateVideoTaskParts { api_key, path: PATH, body: &body }).await
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  // ── Wire-shape tests ──

  #[test]
  fn wire_shape_full_request() {
    let mut request = TextToVideoKling25TurboRequest::new("a fox by a stream");
    request.resolution = Some(VideoResolution::P1080);
    request.aspect_ratio = Some(VideoAspectRatio::Portrait9x16);
    request.duration = Some(ClipDuration::S10);
    assert_eq!(
      text_to_video_kling_2_5_turbo_wire_json(&request).unwrap(),
      json!({
        "prompt": "a fox by a stream",
        "settings": {"resolution": "1080p", "aspect_ratio": "9:16", "duration": 10}
      })
    );
  }

  #[test]
  fn wire_shape_minimal_request() {
    let request = TextToVideoKling25TurboRequest::new("a fox");
    assert_eq!(text_to_video_kling_2_5_turbo_wire_json(&request).unwrap(), json!({"prompt": "a fox"}));
  }

  // ── Validation tests ──

  #[test]
  fn rejects_4k_and_empty_prompt() {
    let mut request = TextToVideoKling25TurboRequest::new("a fox");
    request.resolution = Some(VideoResolution::K4);
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));

    let empty = TextToVideoKling25TurboRequest::new("");
    assert!(matches!(build_body(&empty), Err(KlingClientError::InvalidRequest(_))));
  }
}
