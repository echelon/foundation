use crate::api::requests::video::common::{
  create_video_task, options_wire, validate_prompt, ContentItemWire, CreateVideoTaskParts, SettingsWire,
  VideoCreateBody,
};
use crate::api::types::task_types::created_task::CreatedTask;
use crate::api::types::task_types::request_options::RequestOptions;
use crate::api::types::video_types::character_orientation::CharacterOrientation;
use crate::api::types::video_types::element_ref::ElementRef;
use crate::api::types::video_types::source_audio::SourceAudio;
use crate::api::types::video_types::video_resolution::VideoResolution;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::kling_client_error::KlingClientError;
use crate::error::kling_error::KlingError;

pub(crate) const PATH: &str = "/motion-control/kling-3.0";
const MAX_PROMPT_CHARS: usize = 2500;

/// Kling 3.0 Motion Control: drive a character (appearance image or custom
/// element) with a motion reference video. Output duration follows the
/// reference video; there is no duration or aspect setting.
#[derive(Clone, Debug)]
pub struct MotionControlKling30Request {
  /// ≤2500 chars, optional.
  pub prompt: Option<String>,
  /// Appearance reference image (URL or raw Base64) — the character's upper/
  /// full body with a visible head. Provide this or `element`.
  pub image: Option<String>,
  /// Motion reference video — hosted URL only, `.mp4/.mov`, ≥3s, ≤100MB,
  /// sides 340–3850px. Max 30s with `character_orientation: video`, 10s
  /// with `image`.
  pub video: String,
  /// Alternative character source: at most one custom element (orientation
  /// then always follows the video).
  pub element: Option<ElementRef>,
  /// Required: whether the character's facing follows the image or the video.
  pub character_orientation: CharacterOrientation,
  /// `original` (default — keep the reference video's sound) or `off`.
  pub audio: Option<SourceAudio>,
  /// `720p` (default) or `1080p`.
  pub resolution: Option<VideoResolution>,
  pub options: RequestOptions,
}

impl MotionControlKling30Request {
  pub fn new(video: impl Into<String>, character_orientation: CharacterOrientation) -> Self {
    Self {
      prompt: None,
      image: None,
      video: video.into(),
      element: None,
      character_orientation,
      audio: None,
      resolution: None,
      options: RequestOptions::default(),
    }
  }
}

#[derive(Clone, Debug)]
pub struct MotionControlKling30Args<'a> {
  pub api_key: &'a KlingApiKey,
  pub request: MotionControlKling30Request,
}

pub(crate) fn build_body(request: &MotionControlKling30Request) -> Result<VideoCreateBody<'_>, KlingClientError> {
  if request.video.trim().is_empty() {
    return Err(KlingClientError::InvalidRequest("video (motion reference) must not be empty".into()));
  }
  if request.image.is_none() && request.element.is_none() {
    return Err(KlingClientError::InvalidRequest("provide a character source: image or element".into()));
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
  if let Some(image) = &request.image {
    contents.push(ContentItemWire::Image { url: image });
  }
  contents.push(ContentItemWire::Video { url: &request.video });
  if let Some(element) = &request.element {
    contents.push(ContentItemWire::Element { element_id: &element.element_id, id: &element.id });
  }

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

/// Dry-run: the exact JSON body [`motion_control_kling_3_0`] would send.
pub fn motion_control_kling_3_0_wire_json(
  request: &MotionControlKling30Request,
) -> Result<serde_json::Value, KlingError> {
  let body = build_body(request)?;
  serde_json::to_value(&body).map_err(|e| KlingClientError::RequestSerializationError(e).into())
}

/// `POST /motion-control/kling-3.0` — async; poll the returned task id via
/// [`crate::api::requests::tasks`].
pub async fn motion_control_kling_3_0(args: MotionControlKling30Args<'_>) -> Result<CreatedTask, KlingError> {
  let MotionControlKling30Args { api_key, request } = args;
  let body = build_body(&request)?;
  create_video_task(CreateVideoTaskParts { api_key, path: PATH, body: &body }).await
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  // ── Wire-shape tests ──

  #[test]
  fn wire_shape_image_driven() {
    let mut request =
      MotionControlKling30Request::new("https://example.com/dance.mp4", CharacterOrientation::Video);
    request.prompt = Some("she dances in the rain".to_string());
    request.image = Some("https://example.com/person.png".to_string());
    request.audio = Some(SourceAudio::Off);
    request.resolution = Some(VideoResolution::P1080);
    assert_eq!(
      motion_control_kling_3_0_wire_json(&request).unwrap(),
      json!({
        "contents": [
          {"type": "prompt", "text": "she dances in the rain"},
          {"type": "image", "url": "https://example.com/person.png"},
          {"type": "video", "url": "https://example.com/dance.mp4"}
        ],
        "settings": {"audio": "off", "resolution": "1080p", "character_orientation": "video"}
      })
    );
  }

  #[test]
  fn wire_shape_element_driven() {
    let mut request =
      MotionControlKling30Request::new("https://example.com/dance.mp4", CharacterOrientation::Image);
    request.element = Some(ElementRef { element_id: "829836802793406551".to_string(), id: "Zhang".to_string() });
    assert_eq!(
      motion_control_kling_3_0_wire_json(&request).unwrap(),
      json!({
        "contents": [
          {"type": "video", "url": "https://example.com/dance.mp4"},
          {"type": "element", "element_id": "829836802793406551", "id": "Zhang"}
        ],
        "settings": {"character_orientation": "image"}
      })
    );
  }

  // ── Validation tests ──

  #[test]
  fn rejects_missing_character_source() {
    let request = MotionControlKling30Request::new("https://example.com/dance.mp4", CharacterOrientation::Video);
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }

  #[test]
  fn rejects_empty_video() {
    let mut request = MotionControlKling30Request::new("", CharacterOrientation::Video);
    request.image = Some("https://example.com/p.png".to_string());
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }

  #[test]
  fn rejects_4k() {
    let mut request = MotionControlKling30Request::new("https://example.com/d.mp4", CharacterOrientation::Video);
    request.image = Some("https://example.com/p.png".to_string());
    request.resolution = Some(VideoResolution::K4);
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }
}
