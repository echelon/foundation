use crate::api::requests::video::common::{
  create_video_task, options_wire, validate_duration_range, validate_prompt, CreateVideoTaskParts,
  SettingsWire, VideoCreateBody,
};
use crate::api::types::task_types::created_task::CreatedTask;
use crate::api::types::task_types::request_options::RequestOptions;
use crate::api::types::video_types::native_audio::NativeAudio;
use crate::api::types::video_types::video_aspect_ratio::VideoAspectRatio;
use crate::api::types::video_types::video_resolution::VideoResolution;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::kling_client_error::KlingClientError;
use crate::error::kling_error::KlingError;

pub(crate) const PATH: &str = "/text-to-video/kling-3.0";
const MAX_PROMPT_CHARS: usize = 3072;

/// Kling 3.0 text-to-video: multi-shot (default on), optional native audio,
/// up to 4K.
#[derive(Clone, Debug)]
pub struct TextToVideoKling30Request {
  /// ≤3072 chars (≤2500 recommended). Multi-shot uses the
  /// `"shot n, m, <words>;"` template — up to 6 shots, each ≥1s, durations
  /// summing to `duration`.
  pub prompt: String,
  /// Default true; false suppresses multi-shot output.
  pub multi_shot: Option<bool>,
  /// Default off.
  pub audio: Option<NativeAudio>,
  /// `720p` (default), `1080p`, or `4k`.
  pub resolution: Option<VideoResolution>,
  /// Default 16:9.
  pub aspect_ratio: Option<VideoAspectRatio>,
  /// Any integer 3–15 seconds; default 5.
  pub duration: Option<u32>,
  pub options: RequestOptions,
}

impl TextToVideoKling30Request {
  pub fn new(prompt: impl Into<String>) -> Self {
    Self {
      prompt: prompt.into(),
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
pub struct TextToVideoKling30Args<'a> {
  pub api_key: &'a KlingApiKey,
  pub request: TextToVideoKling30Request,
}

pub(crate) fn build_body(request: &TextToVideoKling30Request) -> Result<VideoCreateBody<'_>, KlingClientError> {
  validate_prompt(&request.prompt, MAX_PROMPT_CHARS)?;
  if let Some(duration) = request.duration {
    validate_duration_range(duration, 3, 15)?;
  }
  Ok(VideoCreateBody {
    prompt: Some(&request.prompt),
    contents: vec![],
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

/// Dry-run: the exact JSON body [`text_to_video_kling_3_0`] would send.
pub fn text_to_video_kling_3_0_wire_json(request: &TextToVideoKling30Request) -> Result<serde_json::Value, KlingError> {
  let body = build_body(request)?;
  serde_json::to_value(&body).map_err(|e| KlingClientError::RequestSerializationError(e).into())
}

/// `POST /text-to-video/kling-3.0` — async; poll the returned task id via
/// [`crate::api::requests::tasks`].
pub async fn text_to_video_kling_3_0(args: TextToVideoKling30Args<'_>) -> Result<CreatedTask, KlingError> {
  let TextToVideoKling30Args { api_key, request } = args;
  let body = build_body(&request)?;
  create_video_task(CreateVideoTaskParts { api_key, path: PATH, body: &body }).await
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  // ── Wire-shape tests ──

  /// Input → exact outbound HTTP body.
  #[test]
  fn wire_shape_full_request() {
    let mut request = TextToVideoKling30Request::new("shot 1, 5, a fox runs; shot 2, 5, it leaps;");
    request.multi_shot = Some(true);
    request.audio = Some(NativeAudio::Native);
    request.resolution = Some(VideoResolution::K4);
    request.aspect_ratio = Some(VideoAspectRatio::Landscape16x9);
    request.duration = Some(10);
    assert_eq!(
      text_to_video_kling_3_0_wire_json(&request).unwrap(),
      json!({
        "prompt": "shot 1, 5, a fox runs; shot 2, 5, it leaps;",
        "settings": {
          "multi_shot": true,
          "audio": "native",
          "resolution": "4k",
          "aspect_ratio": "16:9",
          "duration": 10
        }
      })
    );
  }

  #[test]
  fn wire_shape_minimal_request() {
    let request = TextToVideoKling30Request::new("a fox");
    assert_eq!(text_to_video_kling_3_0_wire_json(&request).unwrap(), json!({"prompt": "a fox"}));
  }

  #[test]
  fn multi_shot_false_is_emitted_not_omitted() {
    let mut request = TextToVideoKling30Request::new("a fox");
    request.multi_shot = Some(false);
    assert_eq!(
      text_to_video_kling_3_0_wire_json(&request).unwrap(),
      json!({"prompt": "a fox", "settings": {"multi_shot": false}})
    );
  }

  // ── Validation tests ──

  #[test]
  fn rejects_overlong_prompt() {
    let request = TextToVideoKling30Request::new("x".repeat(3073));
    assert!(matches!(build_body(&request), Err(KlingClientError::InvalidRequest(_))));
  }

  #[test]
  fn accepts_full_duration_range() {
    for duration in [3, 15] {
      let mut request = TextToVideoKling30Request::new("a fox");
      request.duration = Some(duration);
      assert!(build_body(&request).is_ok(), "duration {duration} should be valid");
    }
  }

  // ── Live API test (ignored — incurs cost) ──
  // cargo test -p kling_client text_to_video_kling_3_0 -- --ignored --nocapture

  #[tokio::test]
  #[ignore] // manually test — requires real API key and incurs costs
  async fn live_test_create() -> anyhow::Result<()> {
    use crate::test_utils::get_test_api_key::get_test_api_key;
    use crate::test_utils::setup_test_logging::setup_test_logging;
    setup_test_logging();

    let api_key = get_test_api_key()?;
    let mut request = TextToVideoKling30Request::new("A serene mountain lake at sunrise, drone shot");
    request.duration = Some(3);
    let created = text_to_video_kling_3_0(TextToVideoKling30Args { api_key: &api_key, request })
      .await
      .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("created task: {} status: {:?}", created.id, created.status);
    Ok(())
  }
}
