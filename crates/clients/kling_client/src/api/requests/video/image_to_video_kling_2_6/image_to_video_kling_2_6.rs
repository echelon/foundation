use crate::api::requests::video::common::{
  create_video_task, options_wire, validate_prompt, ContentItemWire, CreateVideoTaskParts, SettingsWire,
  VideoCreateBody,
};
use crate::api::types::task_types::created_task::CreatedTask;
use crate::api::types::task_types::request_options::RequestOptions;
use crate::api::types::video_types::clip_duration::ClipDuration;
use crate::api::types::video_types::native_audio::NativeAudio;
use crate::api::types::video_types::video_resolution::VideoResolution;
use crate::api::types::video_types::voice_ref::VoiceRef;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::kling_client_error::KlingClientError;
use crate::error::kling_error::KlingError;

pub(crate) const PATH: &str = "/image-to-video/kling-2.6";
const MAX_PROMPT_CHARS: usize = 2500;
const MAX_VOICES: usize = 2;

/// Kling 2.6 image-to-video — the only binding with **voice control**: up to
/// 2 custom voices referenced in the prompt as `@id` (requires native audio,
/// which is 1080p-only). First+last frames are 720p-only, so first+last and
/// native audio are mutually exclusive.
#[derive(Clone, Debug)]
pub struct ImageToVideoKling26Request {
  /// ≤2500 chars. Reference voices as `@id`, e.g. "…@1 whispers softly…".
  pub prompt: Option<String>,
  /// URL or raw Base64; `.jpg/.jpeg/.png`, ≤50MB, sides ≥300px,
  /// aspect 1:2.5–2.5:1.
  pub first_frame: String,
  /// First+last requires `resolution: P720`.
  pub last_frame: Option<String>,
  /// Up to 2 custom voices from the Voice Management API (or presets).
  /// Requires `audio: native`.
  pub voices: Vec<VoiceRef>,
  /// Default off. `native` requires `resolution: P1080`.
  pub audio: Option<NativeAudio>,
  /// `720p` (default) or `1080p`.
  pub resolution: Option<VideoResolution>,
  /// 5s (default) or 10s.
  pub duration: Option<ClipDuration>,
  pub options: RequestOptions,
}

impl ImageToVideoKling26Request {
  pub fn new(first_frame: impl Into<String>) -> Self {
    Self {
      prompt: None,
      first_frame: first_frame.into(),
      last_frame: None,
      voices: vec![],
      audio: None,
      resolution: None,
      duration: None,
      options: RequestOptions::default(),
    }
  }
}

#[derive(Clone, Debug)]
pub struct ImageToVideoKling26Args<'a> {
  pub api_key: &'a KlingApiKey,
  pub request: ImageToVideoKling26Request,
}

pub(crate) fn build_body(request: &ImageToVideoKling26Request) -> Result<VideoCreateBody<'_>, KlingClientError> {
  if request.first_frame.trim().is_empty() {
    return Err(KlingClientError::InvalidRequest("first_frame must not be empty".into()));
  }
  if let Some(prompt) = &request.prompt {
    validate_prompt(prompt, MAX_PROMPT_CHARS)?;
  }
  if request.resolution == Some(VideoResolution::K4) {
    return Err(KlingClientError::InvalidRequest("kling-2.6 supports 720p/1080p only, not 4k".into()));
  }
  if request.voices.len() > MAX_VOICES {
    return Err(KlingClientError::InvalidRequest(format!(
      "at most {MAX_VOICES} voices are supported, got {}",
      request.voices.len()
    )));
  }
  let native_audio = request.audio == Some(NativeAudio::Native);
  if !request.voices.is_empty() && !native_audio {
    return Err(KlingClientError::InvalidRequest("voice control requires audio native".into()));
  }
  if native_audio && request.resolution != Some(VideoResolution::P1080) {
    return Err(KlingClientError::InvalidRequest("native audio on kling-2.6 requires resolution 1080p".into()));
  }
  if request.last_frame.is_some() && request.resolution == Some(VideoResolution::P1080) {
    return Err(KlingClientError::InvalidRequest(
      "first+last frames on kling-2.6 are 720p-only (which also excludes native audio)".into(),
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
  for voice in &request.voices {
    contents.push(ContentItemWire::Voice { voice_id: &voice.voice_id, id: &voice.id });
  }

  Ok(VideoCreateBody {
    prompt: None,
    contents,
    settings: SettingsWire {
      audio: request.audio.map(|a| a.as_str()),
      resolution: request.resolution.map(|r| r.as_str()),
      duration: request.duration.map(|d| d.seconds()),
      ..Default::default()
    }
    .into_option(),
    options: options_wire(&request.options),
  })
}

/// Dry-run: the exact JSON body [`image_to_video_kling_2_6`] would send.
pub fn image_to_video_kling_2_6_wire_json(request: &ImageToVideoKling26Request) -> Result<serde_json::Value, KlingError> {
  let body = build_body(request)?;
  serde_json::to_value(&body).map_err(|e| KlingClientError::RequestSerializationError(e).into())
}

/// `POST /image-to-video/kling-2.6` — async; poll the returned task id via
/// [`crate::api::requests::tasks`].
pub async fn image_to_video_kling_2_6(args: ImageToVideoKling26Args<'_>) -> Result<CreatedTask, KlingError> {
  let ImageToVideoKling26Args { api_key, request } = args;
  let body = build_body(&request)?;
  create_video_task(CreateVideoTaskParts { api_key, path: PATH, body: &body }).await
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  // ── Wire-shape tests ──

  /// The documented voice-control example: two voices, native audio, 1080p.
  #[test]
  fn wire_shape_voice_control() {
    let mut request = ImageToVideoKling26Request::new("https://example.com/first.png");
    request.prompt = Some("@1 whispers softly while @2 laughs".to_string());
    request.voices = vec![
      VoiceRef { voice_id: "891495440020668439".to_string(), id: "1".to_string() },
      VoiceRef { voice_id: "891495440020668440".to_string(), id: "2".to_string() },
    ];
    request.audio = Some(NativeAudio::Native);
    request.resolution = Some(VideoResolution::P1080);
    request.duration = Some(ClipDuration::S10);
    assert_eq!(
      image_to_video_kling_2_6_wire_json(&request).unwrap(),
      json!({
        "contents": [
          {"type": "prompt", "text": "@1 whispers softly while @2 laughs"},
          {"type": "first_frame", "url": "https://example.com/first.png"},
          {"type": "voice", "voice_id": "891495440020668439", "id": "1"},
          {"type": "voice", "voice_id": "891495440020668440", "id": "2"}
        ],
        "settings": {"audio": "native", "resolution": "1080p", "duration": 10}
      })
    );
  }

  #[test]
  fn wire_shape_first_and_last_frame() {
    let mut request = ImageToVideoKling26Request::new("https://example.com/first.png");
    request.last_frame = Some("https://example.com/last.png".to_string());
    request.resolution = Some(VideoResolution::P720);
    assert_eq!(
      image_to_video_kling_2_6_wire_json(&request).unwrap(),
      json!({
        "contents": [
          {"type": "first_frame", "url": "https://example.com/first.png"},
          {"type": "last_frame", "url": "https://example.com/last.png"}
        ],
        "settings": {"resolution": "720p"}
      })
    );
  }

  // ── Validation tests ──

  #[test]
  fn voices_require_native_audio() {
    let mut request = ImageToVideoKling26Request::new("https://example.com/f.png");
    request.voices = vec![VoiceRef { voice_id: "v".to_string(), id: "1".to_string() }];
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }

  #[test]
  fn rejects_more_than_two_voices() {
    let mut request = ImageToVideoKling26Request::new("https://example.com/f.png");
    request.audio = Some(NativeAudio::Native);
    request.resolution = Some(VideoResolution::P1080);
    request.voices = (0..3)
      .map(|i| VoiceRef { voice_id: format!("v{i}"), id: format!("{i}") })
      .collect();
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }

  #[test]
  fn first_plus_last_excludes_1080p_and_native_audio() {
    let mut request = ImageToVideoKling26Request::new("https://example.com/f.png");
    request.last_frame = Some("https://example.com/l.png".to_string());
    request.resolution = Some(VideoResolution::P1080);
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }

  #[test]
  fn native_audio_requires_1080p() {
    let mut request = ImageToVideoKling26Request::new("https://example.com/f.png");
    request.audio = Some(NativeAudio::Native);
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }
}
