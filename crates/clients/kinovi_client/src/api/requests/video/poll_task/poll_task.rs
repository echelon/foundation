use client_core::http_client;
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

/// Lifecycle state of a Kinovi task, mapped from its `waiting/generating/success/fail`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TaskState {
  Pending,
  Running,
  Succeeded,
  Failed,
}

impl TaskState {
  /// Whether polling should stop (the task reached a terminal state).
  pub fn is_terminal(self) -> bool {
    matches!(self, TaskState::Succeeded | TaskState::Failed)
  }
}

/// A task-level error reported inside a successful poll response (`status: "fail"`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PollError {
  /// Provider error code, rendered to a string (Kinovi returns it as string or number).
  pub code: Option<String>,
  /// Human-readable error message.
  pub message: String,
}

/// The resolved state of a polled task.
#[derive(Clone, Debug)]
pub struct PollResult {
  /// Lifecycle state, mapped from Kinovi's `waiting/generating/success/fail`.
  pub state: TaskState,

  /// URL of the generated video (`video/mp4`), present once the task has succeeded.
  pub video_url: Option<String>,

  /// Credits Kinovi reports for the task (`creditsUsed`), when present.
  pub credits_used: Option<f64>,

  /// Task-level failure detail, present when `state` is [`TaskState::Failed`] and
  /// Kinovi returned an `error` object.
  pub error: Option<PollError>,
}

/// Map Kinovi's `status` string onto a [`TaskState`]. Unknown strings are treated as
/// [`TaskState::Failed`] (a terminal, safe-to-stop-polling state).
fn map_status(status: &str) -> TaskState {
  match status {
    "waiting" => TaskState::Pending,
    "generating" => TaskState::Running,
    "success" => TaskState::Succeeded,
    "fail" => TaskState::Failed,
    _ => TaskState::Failed,
  }
}

/// Build a [`PollResult`] from a parsed record-info body. Split out from the HTTP path so it
/// can be unit-tested against fixtures.
fn to_poll_result(body: RecordInfoResponseBody) -> PollResult {
  let state = map_status(&body.status);

  let video_url = body.output.into_iter().flatten().find_map(|o| o.url);

  let error = body.error.map(|e| PollError {
    code: e.code.map(|c| match c {
      serde_json::Value::String(s) => s,
      other => other.to_string(),
    }),
    message: e.message,
  });

  PollResult { state, video_url, credits_used: body.credits_used, error }
}

/// `GET https://kinovi.ai/api/v1/jobs/recordInfo?taskId=…` — poll a previously created task.
/// On success the video URL is at [`PollResult::video_url`].
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
    has_video = result.video_url.is_some(),
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

  #[test]
  fn waiting_maps_to_pending() {
    let r = parse(r#"{ "taskId": "t1", "status": "waiting", "output": [] }"#);
    assert_eq!(r.state, TaskState::Pending);
    assert!(r.video_url.is_none());
    assert!(r.error.is_none());
  }

  #[test]
  fn generating_maps_to_running() {
    let r = parse(r#"{ "taskId": "t1", "status": "generating" }"#);
    assert_eq!(r.state, TaskState::Running);
    assert!(!r.state.is_terminal());
  }

  #[test]
  fn success_maps_to_succeeded_with_video_url() {
    let json = r#"{
      "taskId": "t1",
      "status": "success",
      "output": [
        { "url": "https://cdn.kinovi.ai/videos/abc.mp4", "width": 1280, "height": 720, "seed": 42, "lastFrameImage": "https://cdn.kinovi.ai/frames/abc.png" }
      ],
      "creditsUsed": 37.5,
      "error": null
    }"#;
    let r = parse(json);
    assert_eq!(r.state, TaskState::Succeeded);
    assert!(r.state.is_terminal());
    assert_eq!(r.video_url.as_deref(), Some("https://cdn.kinovi.ai/videos/abc.mp4"));
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
    assert_eq!(r.state, TaskState::Failed);
    assert!(r.state.is_terminal());
    assert!(r.video_url.is_none());
    let err = r.error.expect("should have error");
    assert_eq!(err.code.as_deref(), Some("GENERATION_FAILED"));
    assert_eq!(err.message, "model timed out");
  }

  #[test]
  fn unknown_status_is_treated_as_failed() {
    let r = parse(r#"{ "status": "wat" }"#);
    assert_eq!(r.state, TaskState::Failed);
  }

  #[test]
  fn null_dimensions_do_not_break_parsing() {
    let json = r#"{ "status": "success", "output": [ { "url": "https://cdn.kinovi.ai/v.mp4", "width": null, "height": null, "seed": null } ] }"#;
    let r = parse(json);
    assert_eq!(r.state, TaskState::Succeeded);
    assert!(r.video_url.is_some());
  }

  #[test]
  fn float_dimensions_and_credits_parse() {
    let json = r#"{ "status": "success", "output": [ { "url": "https://cdn.kinovi.ai/v.mp4", "width": 568.5, "height": 1023.4 } ], "creditsUsed": 12.9 }"#;
    let r = parse(json);
    assert_eq!(r.state, TaskState::Succeeded);
    assert_eq!(r.credits_used, Some(12.9));
  }

  #[test]
  fn missing_output_yields_no_video() {
    let r = parse(r#"{ "status": "success" }"#);
    assert_eq!(r.state, TaskState::Succeeded);
    assert!(r.video_url.is_none());
  }

  #[test]
  fn null_url_in_output_yields_no_video() {
    let r = parse(r#"{ "status": "generating", "output": [ { "url": null } ] }"#);
    assert!(r.video_url.is_none());
  }

  #[test]
  fn numeric_error_code_is_stringified() {
    let json = r#"{ "status": "fail", "error": { "code": 402, "message": "insufficient credits" } }"#;
    let r = parse(json);
    let err = r.error.expect("error");
    assert_eq!(err.code.as_deref(), Some("402"));
    assert_eq!(err.message, "insufficient credits");
  }

  #[tokio::test]
  #[ignore] // manually test — requires a real API key and a live taskId
  async fn live_test_poll_task() -> anyhow::Result<()> {
    use crate::test_utils::get_test_api_key::get_test_api_key;
    use crate::test_utils::setup_test_logging::setup_test_logging;
    setup_test_logging();

    let api_key = get_test_api_key()?;
    let task_id = "REPLACE_WITH_REAL_TASK_ID";
    let result = poll_task(PollTaskArgs { api_key: &api_key, task_id })
      .await
      .map_err(|e| anyhow::anyhow!("{}", e))?;
    println!("state={:?} video={:?} credits={:?}", result.state, result.video_url, result.credits_used);
    Ok(())
  }
}
