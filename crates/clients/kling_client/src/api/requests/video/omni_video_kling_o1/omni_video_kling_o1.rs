use crate::api::requests::video::common::{
  create_video_task, options_wire, validate_duration_range, validate_prompt, ContentItemWire,
  CreateVideoTaskParts, SettingsWire, VideoCreateBody,
};
use crate::api::requests::video::omni_video_kling_3_0_omni::omni_video_kling_3_0_omni::OmniReferenceVideo;
use crate::api::types::task_types::created_task::CreatedTask;
use crate::api::types::task_types::request_options::RequestOptions;
use crate::api::types::video_types::element_ref::ElementRef;
use crate::api::types::video_types::refer_image_input::ReferImageInput;
use crate::api::types::video_types::source_audio::SourceAudio;
use crate::api::types::video_types::video_aspect_ratio::VideoAspectRatio;
use crate::api::types::video_types::video_resolution::VideoResolution;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::kling_client_error::KlingClientError;
use crate::error::kling_error::KlingError;

pub(crate) const PATH: &str = "/omni-video/kling-o1";
const MAX_PROMPT_CHARS: usize = 2500;
const MAX_REFER_IMAGES_AND_ELEMENTS_NO_VIDEO: usize = 7;
const MAX_REFER_IMAGES_AND_ELEMENTS_WITH_VIDEO: usize = 4;

/// Kling O1 video generation (omni-style contents). Differences from 3.0
/// Omni: no multi-shot, no native audio (`original`/`off` only), no 4K,
/// duration 3–10s, multi-image elements only (video-character elements are
/// not supported by O1), and first+last frames exclude all other references.
#[derive(Clone, Debug)]
pub struct OmniVideoKlingO1Request {
  /// ≤2500 chars.
  pub prompt: String,
  /// URL or raw Base64. Last-frame-only is unsupported.
  pub first_frame: Option<String>,
  pub last_frame: Option<String>,
  /// Reference images; with elements, ≤7 (≤4 with a reference video).
  pub refer_images: Vec<ReferImageInput>,
  /// At most one reference video (`.mp4/.mov`, 3–10s, sides 700–2160px,
  /// ≤200MB). `Feature` allows a first frame (not last); `Base` forbids both.
  pub reference_video: Option<OmniReferenceVideo>,
  /// Multi-image elements only.
  pub elements: Vec<ElementRef>,
  /// `original` (keep reference-video sound) or `off` (default).
  pub audio: Option<SourceAudio>,
  /// `720p` (default) or `1080p` — no 4K.
  pub resolution: Option<VideoResolution>,
  /// Default 16:9; only meaningful without a first frame or reference video.
  pub aspect_ratio: Option<VideoAspectRatio>,
  /// Integer 3–10 seconds, default 5. With a first frame and no other
  /// references, only 5 or 10 are allowed.
  pub duration: Option<u32>,
  pub options: RequestOptions,
}

impl OmniVideoKlingO1Request {
  pub fn new(prompt: impl Into<String>) -> Self {
    Self {
      prompt: prompt.into(),
      first_frame: None,
      last_frame: None,
      refer_images: vec![],
      reference_video: None,
      elements: vec![],
      audio: None,
      resolution: None,
      aspect_ratio: None,
      duration: None,
      options: RequestOptions::default(),
    }
  }
}

#[derive(Clone, Debug)]
pub struct OmniVideoKlingO1Args<'a> {
  pub api_key: &'a KlingApiKey,
  pub request: OmniVideoKlingO1Request,
}

pub(crate) fn build_body(request: &OmniVideoKlingO1Request) -> Result<VideoCreateBody<'_>, KlingClientError> {
  validate_prompt(&request.prompt, MAX_PROMPT_CHARS)?;
  if request.last_frame.is_some() && request.first_frame.is_none() {
    return Err(KlingClientError::InvalidRequest("last_frame requires a first_frame (last-only is unsupported)".into()));
  }
  if request.resolution == Some(VideoResolution::K4) {
    return Err(KlingClientError::InvalidRequest("kling-o1 supports 720p/1080p only, not 4k".into()));
  }
  let has_first_and_last = request.first_frame.is_some() && request.last_frame.is_some();
  if has_first_and_last && (!request.refer_images.is_empty() || !request.elements.is_empty()) {
    return Err(KlingClientError::InvalidRequest(
      "first+last frames cannot be combined with refer_images or elements on kling-o1".into(),
    ));
  }
  let max_refs = if request.reference_video.is_some() {
    MAX_REFER_IMAGES_AND_ELEMENTS_WITH_VIDEO
  } else {
    MAX_REFER_IMAGES_AND_ELEMENTS_NO_VIDEO
  };
  if request.refer_images.len() + request.elements.len() > max_refs {
    return Err(KlingClientError::InvalidRequest(format!(
      "refer_images + elements must be ≤ {max_refs} here, got {}",
      request.refer_images.len() + request.elements.len()
    )));
  }
  if let Some(duration) = request.duration {
    validate_duration_range(duration, 3, 10)?;
    let first_frame_only = request.first_frame.is_some()
      && request.last_frame.is_none()
      && request.refer_images.is_empty()
      && request.elements.is_empty()
      && request.reference_video.is_none();
    if first_frame_only && duration != 5 && duration != 10 {
      return Err(KlingClientError::InvalidRequest(
        "with only a first frame, kling-o1 allows durations of 5 or 10 seconds".into(),
      ));
    }
  }
  match &request.reference_video {
    Some(OmniReferenceVideo::Feature(_)) => {
      if request.last_frame.is_some() {
        return Err(KlingClientError::InvalidRequest("a feature_video cannot be combined with a last_frame".into()));
      }
    }
    Some(OmniReferenceVideo::Base(_)) => {
      if request.first_frame.is_some() || request.last_frame.is_some() {
        return Err(KlingClientError::InvalidRequest("a base_video cannot be combined with first/last frames".into()));
      }
    }
    None => {
      if request.audio == Some(SourceAudio::Original) {
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

/// Dry-run: the exact JSON body [`omni_video_kling_o1`] would send.
pub fn omni_video_kling_o1_wire_json(request: &OmniVideoKlingO1Request) -> Result<serde_json::Value, KlingError> {
  let body = build_body(request)?;
  serde_json::to_value(&body).map_err(|e| KlingClientError::RequestSerializationError(e).into())
}

/// `POST /omni-video/kling-o1` — async; poll the returned task id via
/// [`crate::api::requests::tasks`].
pub async fn omni_video_kling_o1(args: OmniVideoKlingO1Args<'_>) -> Result<CreatedTask, KlingError> {
  let OmniVideoKlingO1Args { api_key, request } = args;
  let body = build_body(&request)?;
  create_video_task(CreateVideoTaskParts { api_key, path: PATH, body: &body }).await
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;
  use crate::api::types::video_types::refer_video_input::ReferVideoInput;

  // ── Wire-shape tests ──

  #[test]
  fn wire_shape_refer_images_and_element() {
    let mut request = OmniVideoKlingO1Request::new("@obj on a beach");
    request.refer_images =
      vec![ReferImageInput { url: "https://example.com/obj.png".to_string(), id: Some("obj".to_string()) }];
    request.elements = vec![ElementRef { element_id: "829836802793406551".to_string(), id: "Zhang".to_string() }];
    request.resolution = Some(VideoResolution::P1080);
    request.duration = Some(7);
    assert_eq!(
      omni_video_kling_o1_wire_json(&request).unwrap(),
      json!({
        "contents": [
          {"type": "prompt", "text": "@obj on a beach"},
          {"type": "refer_image", "url": "https://example.com/obj.png", "id": "obj"},
          {"type": "element", "element_id": "829836802793406551", "id": "Zhang"}
        ],
        "settings": {"resolution": "1080p", "duration": 7}
      })
    );
  }

  #[test]
  fn wire_shape_minimal_text_only() {
    let request = OmniVideoKlingO1Request::new("a fox");
    assert_eq!(
      omni_video_kling_o1_wire_json(&request).unwrap(),
      json!({"contents": [{"type": "prompt", "text": "a fox"}]})
    );
  }

  // ── Validation tests ──

  #[test]
  fn rejects_4k() {
    let mut request = OmniVideoKlingO1Request::new("a fox");
    request.resolution = Some(VideoResolution::K4);
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }

  #[test]
  fn rejects_duration_over_10() {
    let mut request = OmniVideoKlingO1Request::new("a fox");
    request.duration = Some(11);
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }

  #[test]
  fn first_frame_only_restricts_duration_to_5_or_10() {
    let mut request = OmniVideoKlingO1Request::new("a fox");
    request.first_frame = Some("https://example.com/f.png".to_string());
    request.duration = Some(7);
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));

    request.duration = Some(10);
    assert!(build_body(&request).is_ok());
  }

  #[test]
  fn rejects_first_plus_last_with_other_references() {
    let mut request = OmniVideoKlingO1Request::new("a fox");
    request.first_frame = Some("https://example.com/f.png".to_string());
    request.last_frame = Some("https://example.com/l.png".to_string());
    request.refer_images = vec![ReferImageInput { url: "https://example.com/r.png".to_string(), id: None }];
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }

  #[test]
  fn reference_video_lowers_the_image_budget_to_4() {
    let mut request = OmniVideoKlingO1Request::new("a fox");
    request.reference_video =
      Some(OmniReferenceVideo::Feature(ReferVideoInput { url: "https://example.com/s.mp4".to_string(), id: None }));
    request.refer_images = (0..5)
      .map(|i| ReferImageInput { url: format!("https://example.com/{i}.png"), id: None })
      .collect();
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }

  #[test]
  fn rejects_original_audio_without_reference_video() {
    let mut request = OmniVideoKlingO1Request::new("a fox");
    request.audio = Some(SourceAudio::Original);
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }
}
