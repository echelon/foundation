//! Download a finished Veo video by its operation-result URI. The URI requires
//! the API key header and expires ~2 days after generation — download and
//! re-host promptly.

use tracing::info;

use client_core::http_client;

use crate::api::requests::aistudio_host::GEMINI_USER_AGENT;
use crate::creds::google_aistudio_api_key::GoogleAiStudioApiKey;
use crate::error::classify_google_aistudio_http_error::classify_google_aistudio_http_error;
use crate::error::google_aistudio_error::GoogleAiStudioError;
use crate::error::google_aistudio_generic_api_error::GoogleAiStudioGenericApiError;

/// Top-level argument to [`download_video`].
#[derive(Clone, Debug)]
pub struct DownloadVideoArgs<'a> {
  pub api_key: &'a GoogleAiStudioApiKey,
  /// The `uri` from a succeeded poll
  /// ([`crate::api::requests::videos::poll_video_operation`]).
  pub uri: &'a str,
}

/// `GET {uri}` (with the API key header, following redirects) → raw video bytes.
pub async fn download_video(args: DownloadVideoArgs<'_>) -> Result<Vec<u8>, GoogleAiStudioError> {
  let DownloadVideoArgs { api_key, uri } = args;

  let client = http_client(GEMINI_USER_AGENT);
  let response = client
    .get(uri)
    .header("x-goog-api-key", &api_key.api_key)
    .send()
    .await
    .map_err(GoogleAiStudioGenericApiError::ReqwestError)?;

  let status = response.status();
  if !status.is_success() {
    let body = response.text().await.unwrap_or_default();
    classify_google_aistudio_http_error(status, Some(&body))?;
    return Err(
      GoogleAiStudioGenericApiError::UncategorizedBadResponseWithStatusAndBody { status_code: status, body }.into(),
    );
  }
  let bytes = response.bytes().await.map_err(GoogleAiStudioGenericApiError::ReqwestError)?;
  info!(bytes = bytes.len(), "Veo video downloaded");
  Ok(bytes.to_vec())
}
