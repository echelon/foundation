use client_core::{http_client, Asset, JobState};
use tracing::info;

use crate::api::requests::kinovi_host::{KINOVI_API_BASE_URL, KINOVI_USER_AGENT};
use crate::api::requests::video::poll_task::request_types::RecordInfoResponseBody;
use crate::creds::kinovi_api_key::KinoviApiKey;
use crate::error::classify_kinovi_http_error::classify_kinovi_http_error;
use crate::error::kinovi_error::KinoviError;
use crate::error::kinovi_generic_api_error::KinoviGenericApiError;

/// Top-level argument to [`poll_task`]. Borrows the API key so it never lands in
/// a `{:?}` of the args.
#[derive(Clone, Debug)]
pub struct PollTaskArgs<'a> {
  pub api_key: &'a KinoviApiKey,
  /// The `taskId` returned by
  /// [`crate::api::requests::video::create_task::create_task::create_task`].
  pub task_id: &'a str,
}

/// A task-level error reported inside a successful poll response (`status: "fail"`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PollError {
  /// Provider error code, rendered to a string (Kinovi returns it as string or number).
  pub code: Option<String>,
  /// Human-readable error message.
  pub message: String,
}

/// The resolved state of a polled task, normalized onto [`client_core`] types.
#[derive(Clone, Debug)]
pub struct PollResult {
  /// Lifecycle state, mapped from Kinovi's `waiting/generating/success/fail`.
  pub state: JobState,

  /// The generated video, present once the task has succeeded. Always a
  /// [`Asset::Url`] with `mime = "video/mp4"`.
  pub video: Option<Asset>,

  /// Credits Kinovi reports for the task (`creditsUsed`), when present.
  pub credits_used: Option<f64>,

  /// Task-level failure detail, present when `state` is [`JobState::Failed`] and
  /// Kinovi returned an `error` object.
  pub error: Option<PollError>,
}

/// Map Kinovi's `status` string onto a [`JobState`]. Unknown strings are treated
/// as [`JobState::Failed`] (a terminal, safe-to-stop-polling state).
fn map_status(status: &str) -> JobState {
  match status {
    "waiting" => JobState::Pending,
    "generating" => JobState::Running,
    "success" => JobState::Succeeded,
    "fail" => JobState::Failed,
    _ => JobState::Failed,
  }
}

/// Build a [`PollResult`] from a parsed record-info body. Split out from the HTTP
/// path so it can be unit-tested against fixtures.
fn to_poll_result(body: RecordInfoResponseBody) -> PollResult {
  let state = map_status(&body.status);

  let video = body
    .output
    .into_iter()
    .flatten()
    .find_map(|o| o.url)
    .map(|url| Asset::Url { mime: Some("video/mp4".to_string()), url });

  let error = body.error.map(|e| PollError {
    code: e.code.map(|c| match c {
      serde_json::Value::String(s) => s,
      other => other.to_string(),
    }),
    message: e.message,
  });

  PollResult { state, video, credits_used: body.credits_used, error }
}

/// `GET https://kinovi.ai/api/v1/jobs/recordInfo?taskId=…` — poll a previously
/// created task. Maps Kinovi's status onto [`JobState`]; on success exposes the
/// video URL as an [`Asset::Url`].
pub async fn poll_task(args: PollTaskArgs<'_>) -> Result<PollResult, KinoviError> {
  let PollTaskArgs { api_key, task_id } = args;

  info!(task_id = %task_id, "Kinovi poll_task");

  let url = format!("{}/jobs/recordInfo", KINOVI_API_BASE_URL);
  let client = http_client(KINOVI_USER_AGENT);

  let response = client
    .get(&url)
    .query(&[("taskId", task_id)])
    .bearer_auth(&api_key.api_key)
    .send()
    .await
    .map_err(KinoviGenericApiError::ReqwestError)?;

  let status = response.status();
  let response_body = response.text().await.map_err(KinoviGenericApiError::ReqwestError)?;

  classify_kinovi_http_error(status, Some(&response_body))?;

  let parsed: RecordInfoResponseBody = serde_json::from_str(&response_body)
    .map_err(|err| KinoviGenericApiError::SerdeResponseParseErrorWithBody(err, response_body.clone()))?;

  let result = to_poll_result(parsed);

  info!(
    state = ?result.state,
    has_video = result.video.is_some(),
    credits_used = ?result.credits_used,
    "Kinovi poll_task result"
  );

  Ok(result)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn parse(json: &str) -> PollResult {
    let body: RecordInfoResponseBody = serde_json::from_str(json).expect("parse record info");
    to_poll_result(body)
  }

  // ── Status mapping ──

  #[test]
  fn waiting_maps_to_pending() {
    let r = parse(r#"{ "taskId": "t1", "status": "waiting", "output": [] }"#);
    assert_eq!(r.state, JobState::Pending);
    assert!(r.video.is_none());
    assert!(r.error.is_none());
  }

  #[test]
  fn generating_maps_to_running() {
    let r = parse(r#"{ "taskId": "t1", "status": "generating" }"#);
    assert_eq!(r.state, JobState::Running);
    assert!(!r.state.is_terminal());
  }

  #[test]
  fn success_maps_to_succeeded_with_video_asset() {
    let json = r#"{
      "taskId": "t1",
      "status": "success",
      "output": [
        { "url": "https://cdn.kinovi.ai/videos/abc.mp4", "width": 1280, "height": 720, "seed": 42, "lastFrameImage": "https://cdn.kinovi.ai/frames/abc.png" }
      ],
      "creditsUsed": 37.5,
      "error": null,
      "createTime": "2026-07-19T00:00:00.000Z",
      "completeTime": "2026-07-19T00:01:00.000Z"
    }"#;
    let r = parse(json);
    assert_eq!(r.state, JobState::Succeeded);
    assert!(r.state.is_terminal());
    assert_eq!(
      r.video,
      Some(Asset::Url {
        mime: Some("video/mp4".to_string()),
        url: "https://cdn.kinovi.ai/videos/abc.mp4".to_string(),
      })
    );
    assert_eq!(r.credits_used, Some(37.5));
    assert!(r.error.is_none());
  }

  #[test]
  fn fail_maps_to_failed_with_error() {
    let json = r#"{
      "taskId": "t1",
      "status": "fail",
      "output": [],
      "creditsUsed": 0,
      "error": { "code": "GENERATION_FAILED", "message": "model timed out" }
    }"#;
    let r = parse(json);
    assert_eq!(r.state, JobState::Failed);
    assert!(r.state.is_terminal());
    assert!(r.video.is_none());
    let err = r.error.expect("should have error");
    assert_eq!(err.code.as_deref(), Some("GENERATION_FAILED"));
    assert_eq!(err.message, "model timed out");
  }

  #[test]
  fn unknown_status_is_treated_as_failed() {
    let r = parse(r#"{ "status": "wat" }"#);
    assert_eq!(r.state, JobState::Failed);
  }

  // ── Tolerant deserialization ──

  #[test]
  fn null_dimensions_do_not_break_parsing() {
    let json = r#"{
      "status": "success",
      "output": [ { "url": "https://cdn.kinovi.ai/v.mp4", "width": null, "height": null, "seed": null } ]
    }"#;
    let r = parse(json);
    assert_eq!(r.state, JobState::Succeeded);
    assert!(r.video.is_some());
  }

  #[test]
  fn float_dimensions_and_credits_parse() {
    let json = r#"{
      "status": "success",
      "output": [ { "url": "https://cdn.kinovi.ai/v.mp4", "width": 568.5, "height": 1023.4 } ],
      "creditsUsed": 12.9
    }"#;
    let r = parse(json);
    assert_eq!(r.state, JobState::Succeeded);
    assert_eq!(r.credits_used, Some(12.9));
  }

  #[test]
  fn missing_output_yields_no_video() {
    let r = parse(r#"{ "status": "success" }"#);
    assert_eq!(r.state, JobState::Succeeded);
    assert!(r.video.is_none());
  }

  #[test]
  fn null_url_in_output_yields_no_video() {
    let r = parse(r#"{ "status": "generating", "output": [ { "url": null } ] }"#);
    assert!(r.video.is_none());
  }

  #[test]
  fn numeric_error_code_is_stringified() {
    let json = r#"{ "status": "fail", "error": { "code": 402, "message": "insufficient credits" } }"#;
    let r = parse(json);
    let err = r.error.expect("error");
    assert_eq!(err.code.as_deref(), Some("402"));
    assert_eq!(err.message, "insufficient credits");
  }

  // ── Live API test (ignored) ──

  #[tokio::test]
  #[ignore] // manually test — requires a real API key and a live taskId
  async fn live_test_poll_task() -> anyhow::Result<()> {
    use crate::test_utils::get_test_api_key::get_test_api_key;
    use crate::test_utils::setup_test_logging::setup_test_logging;
    setup_test_logging();

    let api_key = get_test_api_key()?;
    // Replace with a real taskId from a prior create_task call.
    let task_id = "REPLACE_WITH_REAL_TASK_ID";
    let result = poll_task(PollTaskArgs { api_key: &api_key, task_id })
      .await
      .map_err(|e| anyhow::anyhow!("{}", e))?;

    println!("state={:?} video={:?} credits={:?}", result.state, result.video, result.credits_used);
    Ok(())
  }
}
