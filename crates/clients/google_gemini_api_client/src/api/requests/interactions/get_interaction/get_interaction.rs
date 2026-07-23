//! `GET /v1beta/interactions/{id}` — poll a background interaction to
//! completion (~5–10 s cadence works well). Terminal outcomes carry the
//! normalized output items; feed them to the per-family extractors
//! (`music_success_from_interaction`, `omni_video_success_from_interaction`).
//!
//! Docs: <https://ai.google.dev/gemini-api/docs/interactions>

use tracing::info;

use client_core::http_client;

use crate::api::requests::google_gemini_host::{GEMINI_API_BASE_URL, GEMINI_USER_AGENT};
use crate::api::requests::interactions::common::{items_from_body, InteractionHandle, InteractionSuccess};
use crate::creds::google_gemini_api_key::GoogleGeminiApiKey;
use crate::error::classify_google_gemini_http_error::classify_google_gemini_http_error;
use crate::error::google_gemini_error::GoogleGeminiError;
use crate::error::google_gemini_generic_api_error::GoogleGeminiGenericApiError;

/// Top-level argument to [`get_interaction`].
#[derive(Clone, Debug)]
pub struct GetInteractionArgs<'a> {
  pub api_key: &'a GoogleGeminiApiKey,
  pub interaction: &'a InteractionHandle,
}

/// The state of a polled background interaction.
#[derive(Clone, Debug)]
pub enum InteractionPollOutcome {
  /// Still rendering — poll again shortly.
  InProgress,
  /// Done: normalized output items (model_output only — input echoes excluded).
  Completed(InteractionSuccess),
  /// Done, but generation failed.
  Failed { message: String },
  /// Canceled by the user or the system.
  Cancelled,
}

/// Fetch the interaction's current state.
pub async fn get_interaction(args: GetInteractionArgs<'_>) -> Result<InteractionPollOutcome, GoogleGeminiError> {
  let GetInteractionArgs { api_key, interaction } = args;
  let url = format!("{GEMINI_API_BASE_URL}/interactions/{}", interaction.0);

  let client = http_client(GEMINI_USER_AGENT);
  let response = client
    .get(&url)
    .header("x-goog-api-key", &api_key.api_key)
    .send()
    .await
    .map_err(GoogleGeminiGenericApiError::ReqwestError)?;

  let status = response.status();
  let response_body = response.text().await.map_err(GoogleGeminiGenericApiError::ReqwestError)?;
  info!(status = %status, interaction = %interaction.0, "interaction poll");
  classify_google_gemini_http_error(status, Some(&response_body))?;

  let parsed: crate::api::requests::interactions::common::InteractionResponseBody =
    serde_json::from_str(&response_body)
      .map_err(|e| GoogleGeminiGenericApiError::SerdeResponseParseErrorWithBody(e, response_body))?;

  match parsed.status.as_deref() {
    Some("in_progress") | Some("queued") => Ok(InteractionPollOutcome::InProgress),
    Some("cancelled") | Some("canceled") => Ok(InteractionPollOutcome::Cancelled),
    Some("failed") => Ok(InteractionPollOutcome::Failed {
      message: parsed
        .error
        .and_then(|e| e.message)
        .unwrap_or_else(|| "interaction failed without a message".to_string()),
    }),
    // "completed" — and tolerate a missing status on a body that carries output.
    _ => Ok(InteractionPollOutcome::Completed(items_from_body(parsed)?)),
  }
}
