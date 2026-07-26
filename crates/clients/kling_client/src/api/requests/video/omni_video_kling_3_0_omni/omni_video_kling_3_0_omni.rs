use crate::api::requests::video::common::{
  create_video_task, options_wire, validate_duration_range, validate_prompt, ContentItemWire,
  CreateVideoTaskParts, SettingsWire, VideoCreateBody,
};
use crate::api::types::task_types::created_task::CreatedTask;
use crate::api::types::task_types::request_options::RequestOptions;
use crate::api::types::video_types::element_ref::ElementRef;
use crate::api::types::video_types::omni_audio::OmniAudio;
use crate::api::types::video_types::refer_image_input::ReferImageInput;
use crate::api::types::video_types::refer_video_input::ReferVideoInput;
use crate::api::types::video_types::video_aspect_ratio::VideoAspectRatio;
use crate::api::types::video_types::video_resolution::VideoResolution;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::kling_client_error::KlingClientError;
use crate::error::kling_error::KlingError;

pub(crate) const PATH: &str = "/omni-video/kling-3.0-omni";
const MAX_PROMPT_CHARS: usize = 3072;
const MAX_ELEMENTS: usize = 3;
const MAX_REFER_IMAGES_AND_ELEMENTS: usize = 7;

/// The at-most-one reference video of an omni request, and how it's used:
/// `Feature` transfers style/features (multi-shot allowed, audio must stay
/// off); `Base` edits the video itself (no first/last frames, `original`
/// audio allowed, `native` forbidden).
#[derive(Clone, Debug)]
pub enum OmniReferenceVideo {
  Feature(ReferVideoInput),
  Base(ReferVideoInput),
}

/// Kling 3.0 Omni — all-in-one multimodal generation/editing: text, first/
/// last frame, up to 7 reference images, one reference video, custom
/// elements, native audio. Prompt refers to inputs as `@id`.
#[derive(Clone, Debug)]
pub struct OmniVideoKling30OmniRequest {
  /// ≤3072 chars (≤2500 recommended).
  pub prompt: String,
  /// URL or raw Base64. Last-frame-only is unsupported.
  pub first_frame: Option<String>,
  pub last_frame: Option<String>,
  /// Reference images; combined with elements, at most 7 (fewer with a
  /// reference video — see the doc constraints).
  pub refer_images: Vec<ReferImageInput>,
  /// At most one reference video, feature or base.
  pub reference_video: Option<OmniReferenceVideo>,
  /// Up to 3 custom elements from the Element Management API.
  pub elements: Vec<ElementRef>,
  /// Default true; false suppresses multi-shot output.
  pub multi_shot: Option<bool>,
  /// Default off. `original` keeps the reference video's sound.
  pub audio: Option<OmniAudio>,
  /// `720p` (default), `1080p`, or `4k`.
  pub resolution: Option<VideoResolution>,
  /// Default 16:9. Only meaningful without a first frame or reference video
  /// (the frame otherwise follows those inputs).
  pub aspect_ratio: Option<VideoAspectRatio>,
  /// Any integer 3–15 seconds; default 5.
  pub duration: Option<u32>,
  pub options: RequestOptions,
}

impl OmniVideoKling30OmniRequest {
  pub fn new(prompt: impl Into<String>) -> Self {
    Self {
      prompt: prompt.into(),
      first_frame: None,
      last_frame: None,
      refer_images: vec![],
      reference_video: None,
      elements: vec![],
      multi_shot: None,
      audio: None,
      resolution: None,
      aspect_ratio: None,
      duration: None,
      options: RequestOptions::default(),
    }
  }
}

#[derive(Clone, Debug)]
pub struct OmniVideoKling30OmniArgs<'a> {
  pub api_key: &'a KlingApiKey,
  pub request: OmniVideoKling30OmniRequest,
}

pub(crate) fn build_body(request: &OmniVideoKling30OmniRequest) -> Result<VideoCreateBody<'_>, KlingClientError> {
  validate_prompt(&request.prompt, MAX_PROMPT_CHARS)?;
  if request.last_frame.is_some() && request.first_frame.is_none() {
    return Err(KlingClientError::InvalidRequest("last_frame requires a first_frame (last-only is unsupported)".into()));
  }
  if request.elements.len() > MAX_ELEMENTS {
    return Err(KlingClientError::InvalidRequest(format!(
      "at most {MAX_ELEMENTS} elements are supported, got {}",
      request.elements.len()
    )));
  }
  if request.refer_images.len() + request.elements.len() > MAX_REFER_IMAGES_AND_ELEMENTS {
    return Err(KlingClientError::InvalidRequest(format!(
      "refer_images + elements must be ≤ {MAX_REFER_IMAGES_AND_ELEMENTS}, got {}",
      request.refer_images.len() + request.elements.len()
    )));
  }
  if let Some(duration) = request.duration {
    validate_duration_range(duration, 3, 15)?;
  }
  match &request.reference_video {
    Some(OmniReferenceVideo::Feature(_)) => {
      if matches!(request.audio, Some(OmniAudio::Native) | Some(OmniAudio::Original)) {
        return Err(KlingClientError::InvalidRequest("a feature_video requires audio off".into()));
      }
    }
    Some(OmniReferenceVideo::Base(_)) => {
      if request.first_frame.is_some() || request.last_frame.is_some() {
        return Err(KlingClientError::InvalidRequest("a base_video cannot be combined with first/last frames".into()));
      }
      if request.audio == Some(OmniAudio::Native) {
        return Err(KlingClientError::InvalidRequest(
          "a base_video cannot use native audio (use original to keep the source sound)".into(),
        ));
      }
    }
    None => {
      if request.audio == Some(OmniAudio::Original) {
        return Err(KlingClientError::InvalidRequest("audio original requires a reference video".into()));
      }
    }
  }

  let mut contents = vec![ContentItemWire::Prompt { text: &request.prompt }];
  if let Some(first_frame) = &request.first_frame {
    contents.push(ContentItemWire::FirstFrame { url: first_frame });
  }
  if let Some(last_frame) = &request.last_frame {
    contents.push(ContentItemWire::LastFrame { url: last_frame });
  }
  for image in &request.refer_images {
    contents.push(ContentItemWire::ReferImage { url: &image.url, id: image.id.as_deref() });
  }
  match &request.reference_video {
    Some(OmniReferenceVideo::Feature(video)) => {
      contents.push(ContentItemWire::FeatureVideo { url: &video.url, id: video.id.as_deref() });
    }
    Some(OmniReferenceVideo::Base(video)) => {
      contents.push(ContentItemWire::BaseVideo { url: &video.url, id: video.id.as_deref() });
    }
    None => {}
  }
  for element in &request.elements {
    contents.push(ContentItemWire::Element { element_id: &element.element_id, id: &element.id });
  }

  Ok(VideoCreateBody {
    prompt: None,
    contents,
    settings: SettingsWire {
      multi_shot: request.multi_shot,
      audio: request.audio.map(|a| a.as_str()),
      resolution: request.resolution.map(|r| r.as_str()),
      aspect_ratio: request.aspect_ratio.map(|a| a.as_str()),
      duration: request.duration,
      ..Default::default()
    }
    .into_option(),
    options: options_wire(&request.options),
  })
}

/// Dry-run: the exact JSON body [`omni_video_kling_3_0_omni`] would send.
pub fn omni_video_kling_3_0_omni_wire_json(
  request: &OmniVideoKling30OmniRequest,
) -> Result<serde_json::Value, KlingError> {
  let body = build_body(request)?;
  serde_json::to_value(&body).map_err(|e| KlingClientError::RequestSerializationError(e).into())
}

/// `POST /omni-video/kling-3.0-omni` — async; poll the returned task id via
/// [`crate::api::requests::tasks`].
pub async fn omni_video_kling_3_0_omni(args: OmniVideoKling30OmniArgs<'_>) -> Result<CreatedTask, KlingError> {
  let OmniVideoKling30OmniArgs { api_key, request } = args;
  let body = build_body(&request)?;
  create_video_task(CreateVideoTaskParts { api_key, path: PATH, body: &body }).await
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  // ── Wire-shape tests ──

  /// Input → exact outbound HTTP body with every content kind.
  #[test]
  fn wire_shape_kitchen_sink() {
    let mut request = OmniVideoKling30OmniRequest::new("@Zhang wears @image_1's jacket, moving like @video_1");
    request.first_frame = Some("https://example.com/first.png".to_string());
    request.refer_images =
      vec![ReferImageInput { url: "https://example.com/jacket.png".to_string(), id: Some("image_1".to_string()) }];
    request.reference_video = Some(OmniReferenceVideo::Feature(ReferVideoInput {
      url: "https://example.com/style.mp4".to_string(),
      id: Some("video_1".to_string()),
    }));
    request.elements = vec![ElementRef { element_id: "829836802793406551".to_string(), id: "Zhang".to_string() }];
    request.multi_shot = Some(true);
    request.resolution = Some(VideoResolution::P1080);
    request.duration = Some(10);
    assert_eq!(
      omni_video_kling_3_0_omni_wire_json(&request).unwrap(),
      json!({
        "contents": [
          {"type": "prompt", "text": "@Zhang wears @image_1's jacket, moving like @video_1"},
          {"type": "first_frame", "url": "https://example.com/first.png"},
          {"type": "refer_image", "url": "https://example.com/jacket.png", "id": "image_1"},
          {"type": "feature_video", "url": "https://example.com/style.mp4", "id": "video_1"},
          {"type": "element", "element_id": "829836802793406551", "id": "Zhang"}
        ],
        "settings": {"multi_shot": true, "resolution": "1080p", "duration": 10}
      })
    );
  }

  #[test]
  fn wire_shape_base_video_edit() {
    let mut request = OmniVideoKling30OmniRequest::new("replace the sky with aurora");
    request.reference_video = Some(OmniReferenceVideo::Base(ReferVideoInput {
      url: "https://example.com/source.mp4".to_string(),
      id: None,
    }));
    request.audio = Some(OmniAudio::Original);
    assert_eq!(
      omni_video_kling_3_0_omni_wire_json(&request).unwrap(),
      json!({
        "contents": [
          {"type": "prompt", "text": "replace the sky with aurora"},
          {"type": "base_video", "url": "https://example.com/source.mp4"}
        ],
        "settings": {"audio": "original"}
      })
    );
  }

  // ── Validation tests ──

  #[test]
  fn rejects_last_frame_without_first() {
    let mut request = OmniVideoKling30OmniRequest::new("a fox");
    request.last_frame = Some("https://example.com/last.png".to_string());
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }

  #[test]
  fn rejects_feature_video_with_audio_on() {
    let mut request = OmniVideoKling30OmniRequest::new("a fox");
    request.reference_video =
      Some(OmniReferenceVideo::Feature(ReferVideoInput { url: "https://example.com/s.mp4".to_string(), id: None }));
    request.audio = Some(OmniAudio::Native);
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }

  #[test]
  fn rejects_base_video_with_frames_or_native_audio() {
    let mut with_frame = OmniVideoKling30OmniRequest::new("a fox");
    with_frame.reference_video =
      Some(OmniReferenceVideo::Base(ReferVideoInput { url: "https://example.com/s.mp4".to_string(), id: None }));
    with_frame.first_frame = Some("https://example.com/f.png".to_string());
    assert!(matches!(build_body(&with_frame), Err(KlingClientError::InvalidRequest(_))));

    let mut with_native = OmniVideoKling30OmniRequest::new("a fox");
    with_native.reference_video =
      Some(OmniReferenceVideo::Base(ReferVideoInput { url: "https://example.com/s.mp4".to_string(), id: None }));
    with_native.audio = Some(OmniAudio::Native);
    assert!(matches!(build_body(&with_native), Err(KlingClientError::InvalidRequest(_))));
  }

  #[test]
  fn rejects_original_audio_without_reference_video() {
    let mut request = OmniVideoKling30OmniRequest::new("a fox");
    request.audio = Some(OmniAudio::Original);
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }

  #[test]
  fn rejects_too_many_refer_images_plus_elements() {
    let mut request = OmniVideoKling30OmniRequest::new("a fox");
    request.refer_images = (0..6)
      .map(|i| ReferImageInput { url: format!("https://example.com/{i}.png"), id: None })
      .collect();
    request.elements = (0..2)
      .map(|i| ElementRef { element_id: format!("id_{i}"), id: format!("e{i}") })
      .collect();
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }
}
