//! `GET /v1beta/{name}` — fetch a Files-API record (state + metadata). Used to
//! poll Omni's URI-delivered videos to `ACTIVE` before downloading.
//!
//! Docs: <https://ai.google.dev/gemini-api/docs/files>

use serde::Deserialize;

use client_core::http_client;

use crate::api::requests::google_gemini_host::{GEMINI_API_BASE_URL, GEMINI_USER_AGENT};
use crate::creds::google_gemini_api_key::GoogleGeminiApiKey;
use crate::error::classify_google_gemini_http_error::classify_google_gemini_http_error;
use crate::error::google_gemini_error::GoogleGeminiError;
use crate::error::google_gemini_generic_api_error::GoogleGeminiGenericApiError;

/// Top-level argument to [`get_file`].
#[derive(Clone, Debug)]
pub struct GetFileArgs<'a> {
  pub api_key: &'a GoogleGeminiApiKey,
  /// Resource name, e.g. `files/abc-123`.
  pub name: &'a str,
}

/// Lifecycle state of a stored file.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FileState {
  Processing,
  Active,
  Failed,
  /// A state this crate doesn't know yet (forward compat).
  Unknown,
}

/// A Files-API record.
#[derive(Clone, Debug)]
pub struct FileRecord {
  pub name: String,
  pub uri: Option<String>,
  pub mime_type: Option<String>,
  pub state: FileState,
}

#[derive(Deserialize, Debug)]
struct FileBody {
  name: String,
  uri: Option<String>,
  #[serde(rename = "mimeType")]
  mime_type: Option<String>,
  state: Option<String>,
}

/// Files come back sometimes as bare resource names (`files/x`) and sometimes
/// as full `https://…/v1beta/files/x` URIs — accept both.
fn file_url_base(name: &str) -> String {
  if name.starts_with("http") {
    name.to_string()
  } else {
    format!("{GEMINI_API_BASE_URL}/{name}")
  }
}

/// Fetch the file's current state.
pub async fn get_file(args: GetFileArgs<'_>) -> Result<FileRecord, GoogleGeminiError> {
  let GetFileArgs { api_key, name } = args;
  let url = file_url_base(name);

  let client = http_client(GEMINI_USER_AGENT);
  let response = client
    .get(&url)
    .header("x-goog-api-key", &api_key.api_key)
    .send()
    .await
    .map_err(GoogleGeminiGenericApiError::ReqwestError)?;

  let status = response.status();
  let response_body = response.text().await.map_err(GoogleGeminiGenericApiError::ReqwestError)?;
  classify_google_gemini_http_error(status, Some(&response_body))?;

  let parsed: FileBody = serde_json::from_str(&response_body)
    .map_err(|e| GoogleGeminiGenericApiError::SerdeResponseParseErrorWithBody(e, response_body))?;
  let state = match parsed.state.as_deref() {
    Some("PROCESSING") => FileState::Processing,
    Some("ACTIVE") => FileState::Active,
    Some("FAILED") => FileState::Failed,
    _ => FileState::Unknown,
  };
  Ok(FileRecord { name: parsed.name, uri: parsed.uri, mime_type: parsed.mime_type, state })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_states() {
    let body: FileBody =
      serde_json::from_str(r#"{ "name": "files/x", "uri": "https://…/files/x", "state": "ACTIVE" }"#).unwrap();
    assert_eq!(body.state.as_deref(), Some("ACTIVE"));
    let no_state: FileBody = serde_json::from_str(r#"{ "name": "files/y" }"#).unwrap();
    assert!(no_state.state.is_none());
  }
}
