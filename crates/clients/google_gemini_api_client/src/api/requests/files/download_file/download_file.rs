//! `GET /v1beta/{name}:download?alt=media` — download a stored file's bytes
//! (Omni's URI-delivered videos, once `ACTIVE`).
//!
//! Docs: <https://ai.google.dev/gemini-api/docs/omni>

use tracing::info;

use client_core::http_client;

use crate::api::requests::google_gemini_host::{GEMINI_API_BASE_URL, GEMINI_USER_AGENT};
use crate::creds::google_gemini_api_key::GoogleGeminiApiKey;
use crate::error::classify_google_gemini_http_error::classify_google_gemini_http_error;
use crate::error::google_gemini_error::GoogleGeminiError;
use crate::error::google_gemini_generic_api_error::GoogleGeminiGenericApiError;

/// Top-level argument to [`download_file`].
#[derive(Clone, Debug)]
pub struct DownloadFileArgs<'a> {
  pub api_key: &'a GoogleGeminiApiKey,
  /// Resource name, e.g. `files/abc-123`.
  pub name: &'a str,
}

/// Download the file's raw bytes.
///
/// Accepts either a bare resource name (`files/x` — fetched via
/// `:download?alt=media`) or a full `https://…` URI. Omni's URI delivery
/// returns DIRECT download links (a plain GET yields the media bytes, no
/// polling needed), so full URIs are fetched as-is.
pub async fn download_file(args: DownloadFileArgs<'_>) -> Result<Vec<u8>, GoogleGeminiError> {
  let DownloadFileArgs { api_key, name } = args;
  let url = if name.starts_with("http") {
    name.to_string()
  } else {
    format!("{GEMINI_API_BASE_URL}/{name}:download?alt=media")
  };

  let client = http_client(GEMINI_USER_AGENT);
  let response = client
    .get(&url)
    .header("x-goog-api-key", &api_key.api_key)
    .send()
    .await
    .map_err(GoogleGeminiGenericApiError::ReqwestError)?;

  let status = response.status();
  if !status.is_success() {
    let body = response.text().await.unwrap_or_default();
    classify_google_gemini_http_error(status, Some(&body))?;
    return Err(
      GoogleGeminiGenericApiError::UncategorizedBadResponseWithStatusAndBody { status_code: status, body }.into(),
    );
  }
  let bytes = response.bytes().await.map_err(GoogleGeminiGenericApiError::ReqwestError)?;
  info!(bytes = bytes.len(), name, "file downloaded");
  Ok(bytes.to_vec())
}
