//! `GET /{operation_name}` — poll a Veo long-running operation to completion.
//! Recommended cadence: every ~10 seconds. Terminal outcomes are `Succeeded`
//! (with downloadable video URIs — they expire after ~2 days) or `Failed`.
//!
//! Docs: <https://ai.google.dev/gemini-api/docs/veo>

use serde::Deserialize;
use tracing::info;

use client_core::http_client;

use crate::api::requests::gemini_host::{GEMINI_API_BASE_URL, GEMINI_USER_AGENT};
use crate::api::requests::videos::common::VeoOperationName;
use crate::creds::gemini_api_key::GeminiApiKey;
use crate::error::classify_gemini_http_error::classify_gemini_http_error;
use crate::error::gemini_error::GeminiError;
use crate::error::gemini_generic_api_error::GeminiGenericApiError;

/// Top-level argument to [`poll_video_operation`].
#[derive(Clone, Debug)]
pub struct PollVideoOperationArgs<'a> {
  pub api_key: &'a GeminiApiKey,
  pub operation: &'a VeoOperationName,
}

/// One finished video from a succeeded operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VeoGeneratedVideo {
  /// Download URI (send the API key header; see
  /// [`crate::api::requests::files::download_video`]). Expires ~2 days after
  /// generation.
  pub uri: String,
}

/// The state of a polled operation.
#[derive(Clone, Debug)]
pub enum VeoPollOutcome {
  /// Still rendering — poll again in ~10 s.
  Pending,
  /// Done: one entry per generated video.
  Succeeded(Vec<VeoGeneratedVideo>),
  /// Done, but the generation failed (or every video was filtered).
  Failed { code: Option<i64>, message: String },
}

// ── Wire (tolerant of both REST field spellings) ──

#[derive(Deserialize, Debug)]
struct OperationBody {
  #[serde(default)]
  done: bool,
  error: Option<OperationError>,
  response: Option<OperationResponse>,
}

#[derive(Deserialize, Debug)]
struct OperationError {
  code: Option<i64>,
  message: Option<String>,
}

#[derive(Deserialize, Debug)]
struct OperationResponse {
  #[serde(rename = "generateVideoResponse")]
  generate_video_response: Option<GenerateVideoResponse>,
}

#[derive(Deserialize, Debug)]
struct GenerateVideoResponse {
  #[serde(rename = "generatedSamples", alias = "generatedVideos", default)]
  generated_samples: Vec<GeneratedSample>,
}

#[derive(Deserialize, Debug)]
struct GeneratedSample {
  video: Option<VideoRef>,
}

#[derive(Deserialize, Debug)]
struct VideoRef {
  uri: Option<String>,
}

/// Fetch the operation's current state.
pub async fn poll_video_operation(args: PollVideoOperationArgs<'_>) -> Result<VeoPollOutcome, GeminiError> {
  let PollVideoOperationArgs { api_key, operation } = args;
  let url = format!("{GEMINI_API_BASE_URL}/{}", operation.0);

  let client = http_client(GEMINI_USER_AGENT);
  let response = client
    .get(&url)
    .header("x-goog-api-key", &api_key.api_key)
    .send()
    .await
    .map_err(GeminiGenericApiError::ReqwestError)?;

  let status = response.status();
  let response_body = response.text().await.map_err(GeminiGenericApiError::ReqwestError)?;
  info!(status = %status, operation = %operation.0, "Veo operation poll");
  classify_gemini_http_error(status, Some(&response_body))?;

  let parsed: OperationBody = serde_json::from_str(&response_body)
    .map_err(|e| GeminiGenericApiError::SerdeResponseParseErrorWithBody(e, response_body))?;

  if !parsed.done {
    return Ok(VeoPollOutcome::Pending);
  }
  if let Some(err) = parsed.error {
    return Ok(VeoPollOutcome::Failed {
      code: err.code,
      message: err.message.unwrap_or_else(|| "operation failed without a message".to_string()),
    });
  }
  let videos: Vec<VeoGeneratedVideo> = parsed
    .response
    .and_then(|r| r.generate_video_response)
    .map(|g| g.generated_samples)
    .unwrap_or_default()
    .into_iter()
    .filter_map(|s| s.video.and_then(|v| v.uri))
    .map(|uri| VeoGeneratedVideo { uri })
    .collect();
  if videos.is_empty() {
    return Ok(VeoPollOutcome::Failed {
      code: None,
      message: "operation finished with no videos (safety filter?)".to_string(),
    });
  }
  Ok(VeoPollOutcome::Succeeded(videos))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_pending_done_and_failed_shapes() {
    let pending: OperationBody = serde_json::from_str(r#"{ "name": "models/x/operations/y" }"#).unwrap();
    assert!(!pending.done);

    let done: OperationBody = serde_json::from_str(
      r#"{ "name": "n", "done": true,
           "response": { "generateVideoResponse": { "generatedSamples": [ { "video": { "uri": "https://dl/v1" } } ] } } }"#,
    )
    .unwrap();
    assert!(done.done);
    assert_eq!(
      done.response.unwrap().generate_video_response.unwrap().generated_samples[0].video.as_ref().unwrap().uri,
      Some("https://dl/v1".to_string())
    );

    // Newer spelling round-trips through the alias.
    let alias: OperationBody = serde_json::from_str(
      r#"{ "done": true,
           "response": { "generateVideoResponse": { "generatedVideos": [ { "video": { "uri": "https://dl/v2" } } ] } } }"#,
    )
    .unwrap();
    assert_eq!(alias.response.unwrap().generate_video_response.unwrap().generated_samples.len(), 1);

    let failed: OperationBody =
      serde_json::from_str(r#"{ "done": true, "error": { "code": 3, "message": "bad prompt" } }"#).unwrap();
    assert_eq!(failed.error.unwrap().code, Some(3));
  }
}
