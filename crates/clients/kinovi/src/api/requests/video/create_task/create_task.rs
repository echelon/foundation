use client_core::http_client;
use tracing::info;

use crate::api::requests::kinovi_host::{KINOVI_API_BASE_URL, KINOVI_USER_AGENT};
use crate::api::requests::video::create_task::request_types::CreateTaskResponseBody;
use crate::creds::kinovi_api_key::KinoviApiKey;
use crate::error::classify_kinovi_http_error::classify_kinovi_http_error;
use crate::error::kinovi_client_error::KinoviClientError;
use crate::error::kinovi_error::KinoviError;
use crate::error::kinovi_generic_api_error::KinoviGenericApiError;

pub use crate::api::requests::video::create_task::request_types::{CreateTaskRequest, VideoInputs};

/// Top-level argument to [`create_task`]. Borrows the API key separately from the
/// request body so callers can log/persist the [`CreateTaskRequest`] without
/// leaking the credential.
#[derive(Clone, Debug)]
pub struct CreateTaskArgs<'a> {
  pub api_key: &'a KinoviApiKey,
  pub request: CreateTaskRequest,
}

/// `POST https://kinovi.ai/api/v1/jobs/createTask` — submit a Seedance video
/// generation. Kinovi is async: this returns the provider `taskId`, which the
/// caller then polls with
/// [`crate::api::requests::video::poll_task::poll_task::poll_task`].
pub async fn create_task(args: CreateTaskArgs<'_>) -> Result<String, KinoviError> {
  let CreateTaskArgs { api_key, request } = args;

  info!(
    model = %request.model.as_str(),
    duration_seconds = request.inputs.duration_seconds,
    resolution = %request.inputs.resolution.as_str(),
    is_image_to_video = request.inputs.is_image_to_video(),
    "Kinovi create_task"
  );

  // Pre-serialize so a serialization failure is reported as a client error
  // (not swallowed inside reqwest's send path).
  let body_json = serde_json::to_string(&request).map_err(KinoviClientError::RequestSerializationError)?;

  let url = format!("{}/jobs/createTask", KINOVI_API_BASE_URL);
  let client = http_client(KINOVI_USER_AGENT);

  let response = client
    .post(&url)
    .bearer_auth(&api_key.api_key)
    .header("Content-Type", "application/json")
    .body(body_json)
    .send()
    .await
    .map_err(KinoviGenericApiError::ReqwestError)?;

  let status = response.status();
  let response_body = response.text().await.map_err(KinoviGenericApiError::ReqwestError)?;

  info!(status = %status, "Kinovi create_task response");

  classify_kinovi_http_error(status, Some(&response_body))?;

  let parsed: CreateTaskResponseBody = serde_json::from_str(&response_body)
    .map_err(|err| KinoviGenericApiError::SerdeResponseParseErrorWithBody(err, response_body.clone()))?;

  Ok(parsed.task_id)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::api::types::video_types::seedance_model::SeedanceModel;
  use crate::api::types::video_types::video_aspect_ratio::VideoAspectRatio;
  use crate::api::types::video_types::video_resolution::VideoResolution;

  fn text_to_video_request() -> CreateTaskRequest {
    CreateTaskRequest {
      model: SeedanceModel::Seedance20,
      inputs: VideoInputs {
        prompt: "a serene mountain lake at sunrise".to_string(),
        duration_seconds: 5,
        resolution: VideoResolution::P720,
        aspect_ratio: Some(VideoAspectRatio::Landscape16x9),
        image_urls: vec![],
      },
      callback_url: None,
    }
  }

  // ── Wire-shape tests ──

  #[test]
  fn text_to_video_serializes_to_wire_shape() {
    let json = serde_json::to_string(&text_to_video_request()).unwrap();
    assert!(json.contains("\"model\":\"seedance-20\""), "got: {json}");
    assert!(json.contains("\"prompt\":\"a serene mountain lake at sunrise\""), "got: {json}");
    // duration is a STRING of seconds.
    assert!(json.contains("\"duration\":\"5\""), "got: {json}");
    assert!(json.contains("\"outputResolution\":\"720p\""), "got: {json}");
    assert!(json.contains("\"aspectRatio\":\"16:9\""), "got: {json}");
    // text-to-video: no keyframe fields, no callback.
    // (Match on the field key with its quote+colon so the substring "mode" inside
    // "model" doesn't trip the assertion.)
    assert!(!json.contains("\"mode\":"), "t2v must not emit mode; got: {json}");
    assert!(!json.contains("imageUrls"), "t2v must not emit imageUrls; got: {json}");
    assert!(!json.contains("callBackUrl"), "got: {json}");
  }

  #[test]
  fn image_to_video_emits_keyframe_mode_and_urls() {
    let mut req = text_to_video_request();
    req.inputs.image_urls = vec!["https://example.com/first_frame.png".to_string()];
    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("\"mode\":\"keyframe\""), "got: {json}");
    assert!(json.contains("\"imageUrls\":[\"https://example.com/first_frame.png\"]"), "got: {json}");
  }

  #[test]
  fn aspect_ratio_omitted_when_none() {
    let mut req = text_to_video_request();
    req.inputs.aspect_ratio = None;
    let json = serde_json::to_string(&req).unwrap();
    assert!(!json.contains("aspectRatio"), "got: {json}");
  }

  #[test]
  fn callback_url_emitted_when_present() {
    let mut req = text_to_video_request();
    req.callback_url = Some("https://example.com/webhook".to_string());
    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("\"callBackUrl\":\"https://example.com/webhook\""), "got: {json}");
  }

  #[test]
  fn custom_model_serializes_its_id() {
    let mut req = text_to_video_request();
    req.model = SeedanceModel::Custom("seedance-30-preview".to_string());
    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("\"model\":\"seedance-30-preview\""), "got: {json}");
  }

  #[test]
  fn request_does_not_serialize_api_key() {
    let key = KinoviApiKey::new("secret_value_must_not_leak");
    let args = CreateTaskArgs { api_key: &key, request: text_to_video_request() };
    let json = serde_json::to_string(&args.request).unwrap();
    assert!(!json.contains("secret_value_must_not_leak"), "got: {json}");
  }

  // ── Response parsing ──

  #[test]
  fn response_body_deserializes_task_id() {
    let json = r#"{ "taskId": "task_abc123" }"#;
    let parsed: CreateTaskResponseBody = serde_json::from_str(json).unwrap();
    assert_eq!(parsed.task_id, "task_abc123");
  }

  // ── Live API test (ignored — incurs cost) ──

  #[tokio::test]
  #[ignore] // manually test — requires real API key and incurs costs
  async fn live_test_create_task() -> anyhow::Result<()> {
    use crate::test_utils::get_test_api_key::get_test_api_key;
    use crate::test_utils::setup_test_logging::setup_test_logging;
    setup_test_logging();

    let api_key = get_test_api_key()?;
    let task_id = create_task(CreateTaskArgs {
      api_key: &api_key,
      request: CreateTaskRequest {
        model: SeedanceModel::Seedance20Mini,
        inputs: VideoInputs {
          prompt: "A serene mountain lake at sunrise, drone shot".to_string(),
          duration_seconds: 5,
          resolution: VideoResolution::P720,
          aspect_ratio: Some(VideoAspectRatio::Landscape16x9),
          image_urls: vec![],
        },
        callback_url: None,
      },
    })
    .await
    .map_err(|e| anyhow::anyhow!("{}", e))?;

    println!("created task: {task_id}");
    assert!(!task_id.is_empty());
    Ok(())
  }
}
