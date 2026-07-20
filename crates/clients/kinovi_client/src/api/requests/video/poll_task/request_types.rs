use serde::Deserialize;

/// Wire response from `GET /jobs/recordInfo?taskId=…`.
///
/// Kinovi returns `status` as one of `waiting` / `generating` / `success` / `fail`.
/// Numeric fields (`width`, `height`, `seed`, `creditsUsed`) are typed as `f64`
/// because Kinovi has been observed to return them as floats; `null`/missing map
/// to `None` via `#[serde(default)]`.
#[derive(Deserialize, Debug)]
pub(crate) struct RecordInfoResponseBody {
  // Deserialized for completeness / diagnostics; not surfaced in `PollResult`.
  #[allow(dead_code)]
  #[serde(rename = "taskId", default)]
  pub task_id: Option<String>,

  pub status: String,

  #[serde(default)]
  pub output: Option<Vec<RecordInfoOutput>>,

  #[serde(rename = "creditsUsed", default)]
  pub credits_used: Option<f64>,

  #[serde(default)]
  pub error: Option<RecordInfoError>,
}

// `width`/`height`/`seed`/`lastFrameImage` are parsed for tolerance and future
// use but not surfaced in `PollResult` today; only `url` is read.
#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub(crate) struct RecordInfoOutput {
  #[serde(default)]
  pub url: Option<String>,

  #[serde(default)]
  pub width: Option<f64>,

  #[serde(default)]
  pub height: Option<f64>,

  #[serde(default)]
  pub seed: Option<f64>,

  #[serde(rename = "lastFrameImage", default)]
  pub last_frame_image: Option<String>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct RecordInfoError {
  #[serde(default)]
  pub code: Option<serde_json::Value>,

  #[serde(default)]
  pub message: String,
}
