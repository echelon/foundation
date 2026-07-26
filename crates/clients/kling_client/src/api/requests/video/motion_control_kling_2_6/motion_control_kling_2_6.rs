use crate::api::requests::video::common::{
  create_video_task, options_wire, validate_prompt, ContentItemWire, CreateVideoTaskParts, SettingsWire,
  VideoCreateBody,
};
use crate::api::types::task_types::created_task::CreatedTask;
use crate::api::types::task_types::request_options::RequestOptions;
use crate::api::types::video_types::character_orientation::CharacterOrientation;
use crate::api::types::video_types::source_audio::SourceAudio;
use crate::api::types::video_types::video_resolution::VideoResolution;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::kling_client_error::KlingClientError;
use crate::error::kling_error::KlingError;

pub(crate) const PATH: &str = "/motion-control/kling-2.6";
const MAX_PROMPT_CHARS: usize = 2500;

/// Kling 2.6 Motion Control: like 3.0's but without element support — the
/// character source is always an appearance image. Output duration follows
/// the reference video.
#[derive(Clone, Debug)]
pub struct MotionControlKling26Request {
  /// ≤2500 chars, optional.
  pub prompt: Option<String>,
  /// Appearance reference image (URL or raw Base64) — the character's upper/
  /// full body with a visible head.
  pub image: String,
  /// Motion reference video — hosted URL only, `.mp4/.mov`, ≥3s, ≤100MB,
  /// sides 340–3850px. Max 30s with `character_orientation: video`, 10s
  /// with `image`.
  pub video: String,
  /// Required: whether the character's facing follows the image or the video.
  pub character_orientation: CharacterOrientation,
  /// `original` (default — keep the reference video's sound) or `off`.
  pub audio: Option<SourceAudio>,
  /// `720p` (default) or `1080p`.
  pub resolution: Option<VideoResolution>,
  pub options: RequestOptions,
}

impl MotionControlKling26Request {
  pub fn new(
    image: impl Into<String>,
    video: impl Into<String>,
    character_orientation: CharacterOrientation,
  ) -> Self {
    Self {
      prompt: None,
      image: image.into(),
      video: video.into(),
      character_orientation,
      audio: None,
      resolution: None,
      options: RequestOptions::default(),
    }
  }
}

#[derive(Clone, Debug)]
pub struct MotionControlKling26Args<'a> {
  pub api_key: &'a KlingApiKey,
  pub request: MotionControlKling26Request,
}

pub(crate) fn build_body(request: &MotionControlKling26Request) -> Result<VideoCreateBody<'_>, KlingClientError> {
  if request.image.trim().is_empty() {
    return Err(KlingClientError::InvalidRequest("image (appearance reference) must not be empty".into()));
  }
  if request.video.trim().is_empty() {
    return Err(KlingClientError::InvalidRequest("video (motion reference) must not be empty".into()));
  }
  if let Some(prompt) = &request.prompt {
    validate_prompt(prompt, MAX_PROMPT_CHARS)?;
  }
  if request.resolution == Some(VideoResolution::K4) {
    return Err(KlingClientError::InvalidRequest("motion control supports 720p/1080p only, not 4k".into()));
  }

  let mut contents = Vec::new();
  if let Some(prompt) = &request.prompt {
    contents.push(ContentItemWire::Prompt { text: prompt });
  }
  contents.push(ContentItemWire::Image { url: &request.image });
  contents.push(ContentItemWire::Video { url: &request.video });

  Ok(VideoCreateBody {
    prompt: None,
    contents,
    settings: SettingsWire {
      character_orientation: Some(request.character_orientation.as_str()),
      audio: request.audio.map(|a| a.as_str()),
      resolution: request.resolution.map(|r| r.as_str()),
      ..Default::default()
    }
    .into_option(),
    options: options_wire(&request.options),
  })
}

/// Dry-run: the exact JSON body [`motion_control_kling_2_6`] would send.
pub fn motion_control_kling_2_6_wire_json(
  request: &MotionControlKling26Request,
) -> Result<serde_json::Value, KlingError> {
  let body = build_body(request)?;
  serde_json::to_value(&body).map_err(|e| KlingClientError::RequestSerializationError(e).into())
}

/// `POST /motion-control/kling-2.6` — async; poll the returned task id via
/// [`crate::api::requests::tasks`].
pub async fn motion_control_kling_2_6(args: MotionControlKling26Args<'_>) -> Result<CreatedTask, KlingError> {
  let MotionControlKling26Args { api_key, request } = args;
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
    let mut request = MotionControlKling26Request::new(
      "https://example.com/person.png",
      "https://example.com/dance.mp4",
      CharacterOrientation::Image,
    );
    request.prompt = Some("she dances in the rain".to_string());
    request.audio = Some(SourceAudio::Original);
    request.resolution = Some(VideoResolution::P1080);
    assert_eq!(
      motion_control_kling_2_6_wire_json(&request).unwrap(),
      json!({
        "contents": [
          {"type": "prompt", "text": "she dances in the rain"},
          {"type": "image", "url": "https://example.com/person.png"},
          {"type": "video", "url": "https://example.com/dance.mp4"}
        ],
        "settings": {"audio": "original", "resolution": "1080p", "character_orientation": "image"}
      })
    );
  }

  // ── Validation tests ──

  #[test]
  fn rejects_empty_image_or_video() {
    let empty_image =
      MotionControlKling26Request::new("", "https://example.com/d.mp4", CharacterOrientation::Video);
    assert!(matches!(build_body(&empty_image), Err(KlingClientError::InvalidRequest(_))));

    let empty_video =
      MotionControlKling26Request::new("https://example.com/p.png", " ", CharacterOrientation::Video);
    assert!(matches!(build_body(&empty_video), Err(KlingClientError::InvalidRequest(_))));
  }
}
