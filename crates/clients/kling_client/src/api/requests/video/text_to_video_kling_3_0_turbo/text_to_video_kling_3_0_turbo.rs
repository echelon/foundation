use crate::api::requests::video::common::{
  create_video_task, options_wire, validate_duration_range, validate_prompt, CreateVideoTaskParts,
  SettingsWire, VideoCreateBody,
};
use crate::api::types::task_types::created_task::CreatedTask;
use crate::api::types::task_types::request_options::RequestOptions;
use crate::api::types::video_types::video_aspect_ratio::VideoAspectRatio;
use crate::api::types::video_types::video_resolution::VideoResolution;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::kling_client_error::KlingClientError;
use crate::error::kling_error::KlingError;

pub(crate) const PATH: &str = "/text-to-video/kling-3.0-turbo";
const MAX_PROMPT_CHARS: usize = 3072;

/// Kling 3.0 Turbo text-to-video.
///
/// Native audio and multi-shot are prompt-driven on Turbo (no settings
/// switches); multi-shot uses the `"shot n, m, <words>;"` template — up to 6
/// shots, each ≥1s, durations summing to `duration`.
#[derive(Clone, Debug)]
pub struct TextToVideoKling30TurboRequest {
  /// ≤3072 chars (≤2500 recommended); positive and negative description both
  /// go in the prompt text.
  pub prompt: String,
  /// `720p` (default) or `1080p` — Turbo has no 4K.
  pub resolution: Option<VideoResolution>,
  /// Default 16:9.
  pub aspect_ratio: Option<VideoAspectRatio>,
  /// Any integer 3–15 seconds; default 5.
  pub duration: Option<u32>,
  pub options: RequestOptions,
}

impl TextToVideoKling30TurboRequest {
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
pub struct TextToVideoKling30TurboArgs<'a> {
  pub api_key: &'a KlingApiKey,
  pub request: TextToVideoKling30TurboRequest,
}

pub(crate) fn build_body(request: &TextToVideoKling30TurboRequest) -> Result<VideoCreateBody<'_>, KlingClientError> {
  validate_prompt(&request.prompt, MAX_PROMPT_CHARS)?;
  if request.resolution == Some(VideoResolution::K4) {
    return Err(KlingClientError::InvalidRequest("kling-3.0-turbo supports 720p/1080p only, not 4k".into()));
  }
  if let Some(duration) = request.duration {
    validate_duration_range(duration, 3, 15)?;
  }
  Ok(VideoCreateBody {
    prompt: Some(&request.prompt),
    contents: vec![],
    settings: SettingsWire {
      resolution: request.resolution.map(|r| r.as_str()),
      aspect_ratio: request.aspect_ratio.map(|a| a.as_str()),
      duration: request.duration,
      ..Default::default()
    }
    .into_option(),
    options: options_wire(&request.options),
  })
}

/// Dry-run: the exact JSON body [`text_to_video_kling_3_0_turbo`] would send.
pub fn text_to_video_kling_3_0_turbo_wire_json(
  request: &TextToVideoKling30TurboRequest,
) -> Result<serde_json::Value, KlingError> {
  let body = build_body(request)?;
  serde_json::to_value(&body).map_err(|e| KlingClientError::RequestSerializationError(e).into())
}

/// `POST /text-to-video/kling-3.0-turbo` — async; poll the returned task id
/// via [`crate::api::requests::tasks`].
pub async fn text_to_video_kling_3_0_turbo(args: TextToVideoKling30TurboArgs<'_>) -> Result<CreatedTask, KlingError> {
  let TextToVideoKling30TurboArgs { api_key, request } = args;
  let body = build_body(&request)?;
  create_video_task(CreateVideoTaskParts { api_key, path: PATH, body: &body }).await
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;
  use crate::api::types::task_types::request_options::WatermarkInfo;

  // ── Wire-shape tests ──

  /// Input → exact outbound HTTP body.
  #[test]
  fn wire_shape_full_request() {
    let request = TextToVideoKling30TurboRequest {
      prompt: "a hummingbird in slow motion".to_string(),
      resolution: Some(VideoResolution::P1080),
      aspect_ratio: Some(VideoAspectRatio::Portrait9x16),
      duration: Some(8),
      options: RequestOptions {
        callback_url: Some("https://example.com/webhooks/kling".to_string()),
        external_task_id: Some("job_01ABC".to_string()),
        watermark: Some(WatermarkInfo { enabled: false }),
      },
    };
    assert_eq!(
      text_to_video_kling_3_0_turbo_wire_json(&request).unwrap(),
      json!({
        "prompt": "a hummingbird in slow motion",
        "settings": {"resolution": "1080p", "aspect_ratio": "9:16", "duration": 8},
        "options": {
          "callback_url": "https://example.com/webhooks/kling",
          "external_task_id": "job_01ABC",
          "watermark_info": {"enabled": false}
        }
      })
    );
  }

  /// Defaults collapse: bare prompt only — no settings, no options, and
  /// crucially no `contents` array (t2v is top-level prompt).
  #[test]
  fn wire_shape_minimal_request() {
    let request = TextToVideoKling30TurboRequest::new("a fox");
    assert_eq!(
      text_to_video_kling_3_0_turbo_wire_json(&request).unwrap(),
      json!({"prompt": "a fox"})
    );
  }

  // ── Validation tests ──

  #[test]
  fn rejects_4k() {
    let mut request = TextToVideoKling30TurboRequest::new("a fox");
    request.resolution = Some(VideoResolution::K4);
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }

  #[test]
  fn rejects_out_of_range_duration() {
    let mut request = TextToVideoKling30TurboRequest::new("a fox");
    request.duration = Some(16);
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }

  #[test]
  fn rejects_empty_prompt() {
    let request = TextToVideoKling30TurboRequest::new("   ");
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }

  #[test]
  fn request_does_not_serialize_api_key() {
    let key = KlingApiKey::new("secret_value_must_not_leak");
    let args = TextToVideoKling30TurboArgs { api_key: &key, request: TextToVideoKling30TurboRequest::new("a fox") };
    let json = text_to_video_kling_3_0_turbo_wire_json(&args.request).unwrap().to_string();
    assert!(!json.contains("secret_value_must_not_leak"), "got: {json}");
  }

  // ── Live API test (ignored — incurs cost) ──
  // cargo test -p kling_client text_to_video_kling_3_0_turbo -- --ignored --nocapture

  #[tokio::test]
  #[ignore] // manually test — requires real API key and incurs costs
  async fn live_test_create() -> anyhow::Result<()> {
    use crate::test_utils::get_test_api_key::get_test_api_key;
    use crate::test_utils::setup_test_logging::setup_test_logging;
    setup_test_logging();

    let api_key = get_test_api_key()?;
    let mut request = TextToVideoKling30TurboRequest::new("A serene mountain lake at sunrise, drone shot");
    request.duration = Some(3);
    let created = text_to_video_kling_3_0_turbo(TextToVideoKling30TurboArgs { api_key: &api_key, request })
      .await
      .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("created task: {} status: {:?}", created.id, created.status);
    Ok(())
  }
}
