use crate::api::requests::video::common::{
  create_video_task, options_wire, validate_duration_range, validate_prompt, ContentItemWire,
  CreateVideoTaskParts, SettingsWire, VideoCreateBody,
};
use crate::api::types::task_types::created_task::CreatedTask;
use crate::api::types::task_types::request_options::RequestOptions;
use crate::api::types::video_types::element_ref::ElementRef;
use crate::api::types::video_types::native_audio::NativeAudio;
use crate::api::types::video_types::video_resolution::VideoResolution;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::kling_client_error::KlingClientError;
use crate::error::kling_error::KlingError;

pub(crate) const PATH: &str = "/image-to-video/kling-3.0";
const MAX_PROMPT_CHARS: usize = 3072;
const MAX_ELEMENTS: usize = 3;

/// Kling 3.0 image-to-video: first frame (required), optional last frame,
/// up to 3 custom elements referenced in the prompt as `@id`. Frame follows
/// the input image (no aspect setting).
#[derive(Clone, Debug)]
pub struct ImageToVideoKling30Request {
  /// ≤3072 chars (≤2500 recommended). Reference elements as `@id`.
  pub prompt: Option<String>,
  /// URL or raw Base64; `.jpg/.jpeg/.png`, ≤50MB, sides ≥300px,
  /// aspect 1:2.5–2.5:1. Required — last-frame-only is unsupported.
  pub first_frame: String,
  pub last_frame: Option<String>,
  /// Up to 3 custom elements from the Element Management API.
  pub elements: Vec<ElementRef>,
  /// Default true; false suppresses multi-shot output.
  pub multi_shot: Option<bool>,
  /// Default off.
  pub audio: Option<NativeAudio>,
  /// `720p` (default), `1080p`, or `4k`.
  pub resolution: Option<VideoResolution>,
  /// Any integer 3–15 seconds; default 5.
  pub duration: Option<u32>,
  pub options: RequestOptions,
}

impl ImageToVideoKling30Request {
  pub fn new(first_frame: impl Into<String>) -> Self {
    Self {
      prompt: None,
      first_frame: first_frame.into(),
      last_frame: None,
      elements: vec![],
      multi_shot: None,
      audio: None,
      resolution: None,
      duration: None,
      options: RequestOptions::default(),
    }
  }
}

#[derive(Clone, Debug)]
pub struct ImageToVideoKling30Args<'a> {
  pub api_key: &'a KlingApiKey,
  pub request: ImageToVideoKling30Request,
}

pub(crate) fn build_body(request: &ImageToVideoKling30Request) -> Result<VideoCreateBody<'_>, KlingClientError> {
  if request.first_frame.trim().is_empty() {
    return Err(KlingClientError::InvalidRequest("first_frame must not be empty".into()));
  }
  if let Some(prompt) = &request.prompt {
    validate_prompt(prompt, MAX_PROMPT_CHARS)?;
  }
  if request.elements.len() > MAX_ELEMENTS {
    return Err(KlingClientError::InvalidRequest(format!(
      "at most {MAX_ELEMENTS} elements are supported, got {}",
      request.elements.len()
    )));
  }
  if let Some(duration) = request.duration {
    validate_duration_range(duration, 3, 15)?;
  }

  let mut contents = Vec::new();
  if let Some(prompt) = &request.prompt {
    contents.push(ContentItemWire::Prompt { text: prompt });
  }
  contents.push(ContentItemWire::FirstFrame { url: &request.first_frame });
  if let Some(last_frame) = &request.last_frame {
    contents.push(ContentItemWire::LastFrame { url: last_frame });
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
      duration: request.duration,
      ..Default::default()
    }
    .into_option(),
    options: options_wire(&request.options),
  })
}

/// Dry-run: the exact JSON body [`image_to_video_kling_3_0`] would send.
pub fn image_to_video_kling_3_0_wire_json(request: &ImageToVideoKling30Request) -> Result<serde_json::Value, KlingError> {
  let body = build_body(request)?;
  serde_json::to_value(&body).map_err(|e| KlingClientError::RequestSerializationError(e).into())
}

/// `POST /image-to-video/kling-3.0` — async; poll the returned task id via
/// [`crate::api::requests::tasks`].
pub async fn image_to_video_kling_3_0(args: ImageToVideoKling30Args<'_>) -> Result<CreatedTask, KlingError> {
  let ImageToVideoKling30Args { api_key, request } = args;
  let body = build_body(&request)?;
  create_video_task(CreateVideoTaskParts { api_key, path: PATH, body: &body }).await
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  // ── Wire-shape tests ──

  /// Input → exact outbound HTTP body, contents ordered
  /// prompt → first_frame → last_frame → elements.
  #[test]
  fn wire_shape_frames_and_element() {
    let mut request = ImageToVideoKling30Request::new("https://example.com/first.png");
    request.prompt = Some("@Zhang turns toward the camera".to_string());
    request.last_frame = Some("https://example.com/last.png".to_string());
    request.elements = vec![ElementRef { element_id: "829836802793406551".to_string(), id: "Zhang".to_string() }];
    request.audio = Some(NativeAudio::Native);
    request.resolution = Some(VideoResolution::P1080);
    assert_eq!(
      image_to_video_kling_3_0_wire_json(&request).unwrap(),
      json!({
        "contents": [
          {"type": "prompt", "text": "@Zhang turns toward the camera"},
          {"type": "first_frame", "url": "https://example.com/first.png"},
          {"type": "last_frame", "url": "https://example.com/last.png"},
          {"type": "element", "element_id": "829836802793406551", "id": "Zhang"}
        ],
        "settings": {"audio": "native", "resolution": "1080p"}
      })
    );
  }

  #[test]
  fn wire_shape_first_frame_only() {
    let request = ImageToVideoKling30Request::new("https://example.com/first.png");
    assert_eq!(
      image_to_video_kling_3_0_wire_json(&request).unwrap(),
      json!({"contents": [{"type": "first_frame", "url": "https://example.com/first.png"}]})
    );
  }

  // ── Validation tests ──

  #[test]
  fn rejects_more_than_three_elements() {
    let mut request = ImageToVideoKling30Request::new("https://example.com/f.png");
    request.elements = (0..4)
      .map(|i| ElementRef { element_id: format!("id_{i}"), id: format!("e{i}") })
      .collect();
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }

  #[test]
  fn rejects_empty_first_frame() {
    let request = ImageToVideoKling30Request::new("");
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }

  // ── Live API test (ignored — incurs cost) ──
  // cargo test -p kling_client image_to_video_kling_3_0 -- --ignored --nocapture

  #[tokio::test]
  #[ignore] // manually test — requires real API key and incurs costs
  async fn live_test_create() -> anyhow::Result<()> {
    use crate::test_utils::get_test_api_key::get_test_api_key;
    use crate::test_utils::setup_test_logging::setup_test_logging;
    setup_test_logging();

    let api_key = get_test_api_key()?;
    let mut request = ImageToVideoKling30Request::new(test_data::web::image_urls::FOREST_BACKDROP_IMAGE_URL);
    request.prompt = Some("gentle wind moves through the trees".to_string());
    request.duration = Some(3);
    let created = image_to_video_kling_3_0(ImageToVideoKling30Args { api_key: &api_key, request })
      .await
      .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("created task: {} status: {:?}", created.id, created.status);
    Ok(())
  }
}
