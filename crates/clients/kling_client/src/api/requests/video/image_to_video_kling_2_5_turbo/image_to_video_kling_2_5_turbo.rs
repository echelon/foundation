use crate::api::requests::video::common::{
  create_video_task, options_wire, validate_prompt, ContentItemWire, CreateVideoTaskParts, SettingsWire,
  VideoCreateBody,
};
use crate::api::types::task_types::created_task::CreatedTask;
use crate::api::types::task_types::request_options::RequestOptions;
use crate::api::types::video_types::clip_duration::ClipDuration;
use crate::api::types::video_types::video_resolution::VideoResolution;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::kling_client_error::KlingClientError;
use crate::error::kling_error::KlingError;

pub(crate) const PATH: &str = "/image-to-video/kling-2.5-turbo";
const MAX_PROMPT_CHARS: usize = 2500;

/// Kling 2.5 Turbo image-to-video: 5s/10s clips. First+last frames require
/// 1080p (the opposite of 2.6's rule).
#[derive(Clone, Debug)]
pub struct ImageToVideoKling25TurboRequest {
  /// ≤2500 chars.
  pub prompt: Option<String>,
  /// URL or raw Base64; `.jpg/.jpeg/.png`, ≤50MB, sides ≥300px,
  /// aspect 1:2.5–2.5:1.
  pub first_frame: String,
  /// First+last requires `resolution: P1080`.
  pub last_frame: Option<String>,
  /// `720p` (default) or `1080p`.
  pub resolution: Option<VideoResolution>,
  /// 5s (default) or 10s.
  pub duration: Option<ClipDuration>,
  pub options: RequestOptions,
}

impl ImageToVideoKling25TurboRequest {
  pub fn new(first_frame: impl Into<String>) -> Self {
    Self {
      prompt: None,
      first_frame: first_frame.into(),
      last_frame: None,
      resolution: None,
      duration: None,
      options: RequestOptions::default(),
    }
  }
}

#[derive(Clone, Debug)]
pub struct ImageToVideoKling25TurboArgs<'a> {
  pub api_key: &'a KlingApiKey,
  pub request: ImageToVideoKling25TurboRequest,
}

pub(crate) fn build_body(request: &ImageToVideoKling25TurboRequest) -> Result<VideoCreateBody<'_>, KlingClientError> {
  if request.first_frame.trim().is_empty() {
    return Err(KlingClientError::InvalidRequest("first_frame must not be empty".into()));
  }
  if let Some(prompt) = &request.prompt {
    validate_prompt(prompt, MAX_PROMPT_CHARS)?;
  }
  if request.resolution == Some(VideoResolution::K4) {
    return Err(KlingClientError::InvalidRequest("kling-2.5-turbo supports 720p/1080p only, not 4k".into()));
  }
  if request.last_frame.is_some() && request.resolution != Some(VideoResolution::P1080) {
    return Err(KlingClientError::InvalidRequest(
      "first+last frames on kling-2.5-turbo require resolution 1080p".into(),
    ));
  }

  let mut contents = Vec::new();
  if let Some(prompt) = &request.prompt {
    contents.push(ContentItemWire::Prompt { text: prompt });
  }
  contents.push(ContentItemWire::FirstFrame { url: &request.first_frame });
  if let Some(last_frame) = &request.last_frame {
    contents.push(ContentItemWire::LastFrame { url: last_frame });
  }

  Ok(VideoCreateBody {
    prompt: None,
    contents,
    settings: SettingsWire {
      resolution: request.resolution.map(|r| r.as_str()),
      duration: request.duration.map(|d| d.seconds()),
      ..Default::default()
    }
    .into_option(),
    options: options_wire(&request.options),
  })
}

/// Dry-run: the exact JSON body [`image_to_video_kling_2_5_turbo`] would send.
pub fn image_to_video_kling_2_5_turbo_wire_json(
  request: &ImageToVideoKling25TurboRequest,
) -> Result<serde_json::Value, KlingError> {
  let body = build_body(request)?;
  serde_json::to_value(&body).map_err(|e| KlingClientError::RequestSerializationError(e).into())
}

/// `POST /image-to-video/kling-2.5-turbo` — async; poll the returned task id
/// via [`crate::api::requests::tasks`].
pub async fn image_to_video_kling_2_5_turbo(
  args: ImageToVideoKling25TurboArgs<'_>,
) -> Result<CreatedTask, KlingError> {
  let ImageToVideoKling25TurboArgs { api_key, request } = args;
  let body = build_body(&request)?;
  create_video_task(CreateVideoTaskParts { api_key, path: PATH, body: &body }).await
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  // ── Wire-shape tests ──

  #[test]
  fn wire_shape_first_and_last_frame() {
    let mut request = ImageToVideoKling25TurboRequest::new("https://example.com/first.png");
    request.prompt = Some("smooth transition".to_string());
    request.last_frame = Some("https://example.com/last.png".to_string());
    request.resolution = Some(VideoResolution::P1080);
    request.duration = Some(ClipDuration::S5);
    assert_eq!(
      image_to_video_kling_2_5_turbo_wire_json(&request).unwrap(),
      json!({
        "contents": [
          {"type": "prompt", "text": "smooth transition"},
          {"type": "first_frame", "url": "https://example.com/first.png"},
          {"type": "last_frame", "url": "https://example.com/last.png"}
        ],
        "settings": {"resolution": "1080p", "duration": 5}
      })
    );
  }

  // ── Validation tests ──

  /// 2.5 Turbo's first+last rule is the OPPOSITE of 2.6's: it requires
  /// 1080p, not 720p.
  #[test]
  fn first_plus_last_requires_1080p() {
    let mut request = ImageToVideoKling25TurboRequest::new("https://example.com/f.png");
    request.last_frame = Some("https://example.com/l.png".to_string());
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));

    request.resolution = Some(VideoResolution::P720);
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));

    request.resolution = Some(VideoResolution::P1080);
    assert!(build_body(&request).is_ok());
  }

  #[test]
  fn rejects_empty_first_frame() {
    let request = ImageToVideoKling25TurboRequest::new("");
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }
}
