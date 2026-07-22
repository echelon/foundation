//! Files API resumable upload — the PREFERRED way to hand Google an input
//! image (the Gemini API does not fetch public HTTP URLs, and inline base64 is
//! grossly inefficient encoding). Uploads are free; files persist ~48 hours and
//! are referenced in requests via their `file.uri`
//! ([`crate::api::types::image_types::image_input::ImageInput::FileUri`]).
//!
//! Two-step protocol: a `start` call carrying metadata returns an upload URL in
//! the `x-goog-upload-url` header; the bytes then POST to that URL with
//! `upload, finalize`.
//!
//! Docs: <https://ai.google.dev/gemini-api/docs/files>

use serde::{Deserialize, Serialize};
use tracing::info;

use client_core::http_client;

use crate::api::requests::gemini_host::{GEMINI_UPLOAD_BASE_URL, GEMINI_USER_AGENT};
use crate::creds::gemini_api_key::GeminiApiKey;
use crate::error::classify_gemini_http_error::classify_gemini_http_error;
use crate::error::gemini_client_error::GeminiClientError;
use crate::error::gemini_error::GeminiError;
use crate::error::gemini_generic_api_error::GeminiGenericApiError;

/// Top-level argument to [`upload_image_file`].
#[derive(Clone, Debug)]
pub struct UploadImageFileArgs<'a> {
  pub api_key: &'a GeminiApiKey,
  pub request: UploadImageFileRequest,
}

/// The upload itself (no credential inside).
#[derive(Clone, Debug, Serialize)]
pub struct UploadImageFileRequest {
  /// Raw image bytes (≤ 2 GB per file; 20 GB per project).
  #[serde(skip_serializing)]
  pub bytes: Vec<u8>,
  /// MIME type, e.g. `"image/png"`.
  pub mime: String,
  /// Optional human label shown in file listings.
  pub display_name: Option<String>,
}

/// A stored file reference. Persists ~48 h; pass `uri` (+ mime) as a
/// `file_data` part or Veo `fileUri`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UploadedFileRef {
  /// Resource name, e.g. `files/abc-123`.
  pub name: String,
  /// The URI requests reference.
  pub uri: String,
  /// MIME type as recorded by the Files API.
  pub mime_type: String,
}

// ── Wire ──

#[derive(Serialize, Debug)]
pub(crate) struct StartUploadBody {
  pub file: StartUploadFile,
}

#[derive(Serialize, Debug)]
pub(crate) struct StartUploadFile {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub display_name: Option<String>,
}

#[derive(Deserialize, Debug)]
struct FinalizeResponseBody {
  file: FinalizeFile,
}

#[derive(Deserialize, Debug)]
struct FinalizeFile {
  name: String,
  uri: String,
  #[serde(rename = "mimeType")]
  mime_type: String,
}

/// Upload image bytes to the Files API and return the reference to use in
/// generate requests.
pub async fn upload_image_file(args: UploadImageFileArgs<'_>) -> Result<UploadedFileRef, GeminiError> {
  let UploadImageFileArgs { api_key, request } = args;
  if request.bytes.is_empty() {
    return Err(GeminiClientError::InvalidRequest("upload_image_file: empty bytes".to_string()).into());
  }

  let client = http_client(GEMINI_USER_AGENT);

  // Step 1: start — metadata only; the upload URL comes back in a header.
  let start_body = StartUploadBody { file: StartUploadFile { display_name: request.display_name.clone() } };
  let start_json = serde_json::to_string(&start_body).map_err(GeminiClientError::RequestSerializationError)?;
  let start = client
    .post(format!("{GEMINI_UPLOAD_BASE_URL}/files"))
    .header("x-goog-api-key", &api_key.api_key)
    .header("X-Goog-Upload-Protocol", "resumable")
    .header("X-Goog-Upload-Command", "start")
    .header("X-Goog-Upload-Header-Content-Length", request.bytes.len().to_string())
    .header("X-Goog-Upload-Header-Content-Type", &request.mime)
    .header("Content-Type", "application/json")
    .body(start_json)
    .send()
    .await
    .map_err(GeminiGenericApiError::ReqwestError)?;

  let start_status = start.status();
  let upload_url = start
    .headers()
    .get("x-goog-upload-url")
    .and_then(|v| v.to_str().ok())
    .map(str::to_string);
  let Some(upload_url) = upload_url else {
    let body = start.text().await.unwrap_or_default();
    classify_gemini_http_error(start_status, Some(&body))?;
    return Err(
      GeminiGenericApiError::UncategorizedBadResponseWithStatusAndBody { status_code: start_status, body }
        .into(),
    );
  };

  // Step 2: bytes — upload + finalize in one shot.
  let finalize = client
    .post(&upload_url)
    .header("X-Goog-Upload-Offset", "0")
    .header("X-Goog-Upload-Command", "upload, finalize")
    .body(request.bytes.clone())
    .send()
    .await
    .map_err(GeminiGenericApiError::ReqwestError)?;

  let status = finalize.status();
  let response_body = finalize.text().await.map_err(GeminiGenericApiError::ReqwestError)?;
  info!(status = %status, bytes = request.bytes.len(), "Files API upload finalized");
  classify_gemini_http_error(status, Some(&response_body))?;

  let parsed: FinalizeResponseBody = serde_json::from_str(&response_body)
    .map_err(|e| GeminiGenericApiError::SerdeResponseParseErrorWithBody(e, response_body))?;
  Ok(UploadedFileRef { name: parsed.file.name, uri: parsed.file.uri, mime_type: parsed.file.mime_type })
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  /// The start-call metadata body (the bytes travel in step 2, never as JSON).
  #[test]
  fn start_metadata_wire_shape() {
    let body = StartUploadBody { file: StartUploadFile { display_name: Some("edit input".to_string()) } };
    assert_eq!(serde_json::to_value(&body).unwrap(), json!({ "file": { "display_name": "edit input" } }));
    let bare = StartUploadBody { file: StartUploadFile { display_name: None } };
    assert_eq!(serde_json::to_value(&bare).unwrap(), json!({ "file": {} }));
  }

  /// Live upload — FREE (the Files API costs nothing), needs
  /// `external/credentials/gemini_api_key.txt`.
  #[tokio::test]
  #[ignore] // live — needs credentials (uploads are free)
  async fn live_upload_image_file() -> anyhow::Result<()> {
    crate::test_utils::setup_test_logging::setup_test_logging();
    let api_key = crate::test_utils::get_test_api_key::get_test_api_key()?;
    // A 1×1 red PNG.
    let png: &[u8] = &[
      0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00,
      0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, 0xDE, 0x00, 0x00, 0x00,
      0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, 0x00, 0x00, 0x03, 0x00, 0x01, 0x9E,
      0x18, 0x18, 0x8F, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];
    let uploaded = upload_image_file(UploadImageFileArgs {
      api_key: &api_key,
      request: UploadImageFileRequest {
        bytes: png.to_vec(),
        mime: "image/png".to_string(),
        display_name: Some("live-test 1x1".to_string()),
      },
    })
    .await?;
    assert!(uploaded.name.starts_with("files/"), "unexpected name: {}", uploaded.name);
    assert!(uploaded.uri.contains("/files/"), "unexpected uri: {}", uploaded.uri);
    Ok(())
  }
}
