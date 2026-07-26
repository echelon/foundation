use crate::api::requests::video::common::{
  create_video_task, options_wire, validate_duration_range, validate_prompt, ContentItemWire,
  CreateVideoTaskParts, SettingsWire, VideoCreateBody,
};
use crate::api::types::task_types::created_task::CreatedTask;
use crate::api::types::task_types::request_options::RequestOptions;
use crate::api::types::video_types::video_resolution::VideoResolution;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::kling_client_error::KlingClientError;
use crate::error::kling_error::KlingError;

pub(crate) const PATH: &str = "/image-to-video/kling-3.0-turbo";
const MAX_PROMPT_CHARS: usize = 2500;

/// Kling 3.0 Turbo image-to-video: first frame only (no last frame), frame
/// follows the input image (no aspect setting).
#[derive(Clone, Debug)]
pub struct ImageToVideoKling30TurboRequest {
  /// ≤2500 chars.
  pub prompt: Option<String>,
  /// URL or raw Base64; `.jpg/.jpeg/.png`, ≤50MB, sides ≥300px,
  /// aspect 1:2.5–2.5:1.
  pub first_frame: String,
  /// `720p` (default) or `1080p` — Turbo has no 4K.
  pub resolution: Option<VideoResolution>,
  /// Any integer 3–15 seconds; default 5.
  pub duration: Option<u32>,
  pub options: RequestOptions,
}

impl ImageToVideoKling30TurboRequest {
  pub fn new(first_frame: impl Into<String>) -> Self {
    Self {
      prompt: None,
      first_frame: first_frame.into(),
      resolution: None,
      duration: None,
      options: RequestOptions::default(),
    }
  }
}

#[derive(Clone, Debug)]
pub struct ImageToVideoKling30TurboArgs<'a> {
  pub api_key: &'a KlingApiKey,
  pub request: ImageToVideoKling30TurboRequest,
}

pub(crate) fn build_body(request: &ImageToVideoKling30TurboRequest) -> Result<VideoCreateBody<'_>, KlingClientError> {
  if request.first_frame.trim().is_empty() {
    return Err(KlingClientError::InvalidRequest("first_frame must not be empty".into()));
  }
  if let Some(prompt) = &request.prompt {
    validate_prompt(prompt, MAX_PROMPT_CHARS)?;
  }
  if request.resolution == Some(VideoResolution::K4) {
    return Err(KlingClientError::InvalidRequest("kling-3.0-turbo supports 720p/1080p only, not 4k".into()));
  }
  if let Some(duration) = request.duration {
    validate_duration_range(duration, 3, 15)?;
  }

  let mut contents = Vec::new();
  if let Some(prompt) = &request.prompt {
    contents.push(ContentItemWire::Prompt { text: prompt });
  }
  contents.push(ContentItemWire::FirstFrame { url: &request.first_frame });

  Ok(VideoCreateBody {
    prompt: None,
    contents,
    settings: SettingsWire {
      resolution: request.resolution.map(|r| r.as_str()),
      duration: request.duration,
      ..Default::default()
    }
    .into_option(),
    options: options_wire(&request.options),
  })
}

/// Dry-run: the exact JSON body [`image_to_video_kling_3_0_turbo`] would send.
pub fn image_to_video_kling_3_0_turbo_wire_json(
  request: &ImageToVideoKling30TurboRequest,
) -> Result<serde_json::Value, KlingError> {
  let body = build_body(request)?;
  serde_json::to_value(&body).map_err(|e| KlingClientError::RequestSerializationError(e).into())
}

/// `POST /image-to-video/kling-3.0-turbo` — async; poll the returned task id
/// via [`crate::api::requests::tasks`].
pub async fn image_to_video_kling_3_0_turbo(args: ImageToVideoKling30TurboArgs<'_>) -> Result<CreatedTask, KlingError> {
  let ImageToVideoKling30TurboArgs { api_key, request } = args;
  let body = build_body(&request)?;
  create_video_task(CreateVideoTaskParts { api_key, path: PATH, body: &body }).await
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  // ── Wire-shape tests ──

  /// Input → exact outbound HTTP body: prompt and first frame as `contents`
  /// items, never a top-level prompt.
  #[test]
  fn wire_shape_prompt_and_first_frame() {
    let mut request = ImageToVideoKling30TurboRequest::new("https://example.com/first.png");
    request.prompt = Some("the camera pulls back slowly".to_string());
    request.resolution = Some(VideoResolution::P1080);
    request.duration = Some(10);
    assert_eq!(
      image_to_video_kling_3_0_turbo_wire_json(&request).unwrap(),
      json!({
        "contents": [
          {"type": "prompt", "text": "the camera pulls back slowly"},
          {"type": "first_frame", "url": "https://example.com/first.png"}
        ],
        "settings": {"resolution": "1080p", "duration": 10}
      })
    );
  }

  /// No prompt: contents carry only the first frame.
  #[test]
  fn wire_shape_frame_only() {
    let request = ImageToVideoKling30TurboRequest::new("https://example.com/first.png");
    assert_eq!(
      image_to_video_kling_3_0_turbo_wire_json(&request).unwrap(),
      json!({
        "contents": [{"type": "first_frame", "url": "https://example.com/first.png"}]
      })
    );
  }

  // ── Validation tests ──

  #[test]
  fn rejects_empty_first_frame() {
    let request = ImageToVideoKling30TurboRequest::new("  ");
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }

  #[test]
  fn rejects_4k() {
    let mut request = ImageToVideoKling30TurboRequest::new("https://example.com/f.png");
    request.resolution = Some(VideoResolution::K4);
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }

  #[test]
  fn rejects_out_of_range_duration() {
    let mut request = ImageToVideoKling30TurboRequest::new("https://example.com/f.png");
    request.duration = Some(2);
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }

  // ── Live API test (ignored — incurs cost) ──
  // cargo test -p kling_client image_to_video_kling_3_0_turbo -- --ignored --nocapture

  #[tokio::test]
  #[ignore] // manually test — requires real API key and incurs costs
  async fn live_test_create() -> anyhow::Result<()> {
    use crate::test_utils::get_test_api_key::get_test_api_key;
    use crate::test_utils::setup_test_logging::setup_test_logging;
    setup_test_logging();

    let api_key = get_test_api_key()?;
    let mut request = ImageToVideoKling30TurboRequest::new(test_data::web::image_urls::FOREST_BACKDROP_IMAGE_URL);
    request.prompt = Some("gentle wind moves through the trees".to_string());
    request.duration = Some(3);
    let created = image_to_video_kling_3_0_turbo(ImageToVideoKling30TurboArgs { api_key: &api_key, request })
      .await
      .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("created task: {} status: {:?}", created.id, created.status);
    Ok(())
  }
}
