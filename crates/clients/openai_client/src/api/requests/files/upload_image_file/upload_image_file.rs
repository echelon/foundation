//! `POST /v1/files` — upload an image for later use as an edit input (`purpose: vision`).
//!
//! The FALLBACK path: we strongly prefer handing OpenAI a publicly accessible URL
//! ([`InputImageSource::Url`](crate::api::types::image_types::input_image_source::InputImageSource)),
//! which ships no image bytes per generation request. Use this only for sources that can't
//! be public. Uploads themselves are free (storage is metered generously); there is no cost
//! calculator here on purpose.

use reqwest::multipart::{Form, Part};
use tracing::info;

use crate::api::requests::openai_host::{with_auth, OPENAI_API_BASE_URL};
use crate::api::types::image_types::input_image_source::InputImageSource;
use crate::api::types::image_types::output_format::OutputFormat;
use crate::creds::openai_api_key::OpenAiApiKey;
use crate::error::classify_openai_http_error::classify_openai_http_error;
use crate::error::openai_client_error::OpenAiClientError;
use crate::error::openai_error::OpenAiError;
use crate::error::openai_generic_api_error::OpenAiGenericApiError;

// ── Public args ──

#[derive(Debug)]
pub struct UploadImageFileArgs<'a> {
  pub api_key: &'a OpenAiApiKey,
  pub request: UploadImageFileRequest,
}

pub struct UploadImageFileRequest {
  pub bytes: Vec<u8>,
  pub format: OutputFormat,
  /// Filename shown in the OpenAI dashboard (extension should match `format`).
  pub filename: String,
}

// Debug elides the bytes.
impl std::fmt::Debug for UploadImageFileRequest {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("UploadImageFileRequest")
      .field("len", &self.bytes.len())
      .field("format", &self.format)
      .field("filename", &self.filename)
      .finish()
  }
}

impl UploadImageFileRequest {
  pub fn png(bytes: Vec<u8>, filename: impl Into<String>) -> Self {
    Self { bytes, format: OutputFormat::Png, filename: filename.into() }
  }
  pub fn jpeg(bytes: Vec<u8>, filename: impl Into<String>) -> Self {
    Self { bytes, format: OutputFormat::Jpeg, filename: filename.into() }
  }
  pub fn webp(bytes: Vec<u8>, filename: impl Into<String>) -> Self {
    Self { bytes, format: OutputFormat::Webp, filename: filename.into() }
  }
}

// ── Public response ──

/// The uploaded file's id (`file-…`). Convert into an edit input with `into_source()`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UploadedImageFile {
  pub file_id: String,
  pub bytes: u64,
}

impl UploadedImageFile {
  pub fn into_source(self) -> InputImageSource {
    InputImageSource::FileId(self.file_id)
  }
}

// ── Wire ──

#[derive(serde::Deserialize)]
struct FileWire {
  id: String,
  #[serde(default)]
  bytes: u64,
}

// ── Implementation ──

pub async fn upload_image_file(args: UploadImageFileArgs<'_>) -> Result<UploadedImageFile, OpenAiError> {
  let req = args.request;
  if req.bytes.is_empty() {
    return Err(OpenAiClientError::InvalidRequest("image bytes must not be empty".into()).into());
  }

  info!(len = req.bytes.len(), filename = %req.filename, "openai upload image file");

  let part = Part::bytes(req.bytes)
    .file_name(req.filename)
    .mime_str(req.format.mime())
    .map_err(|e| OpenAiClientError::InvalidRequest(format!("bad mime: {e}")))?;
  let form = Form::new().text("purpose", "vision").part("file", part);

  let client = client_core::http_client("foundation/openai-client");
  let builder = client.post(format!("{OPENAI_API_BASE_URL}/v1/files")).multipart(form);

  let response = with_auth(builder, args.api_key)
    .send()
    .await
    .map_err(OpenAiGenericApiError::ReqwestError)?;

  let status = response.status();
  let text = response.text().await.map_err(OpenAiGenericApiError::ReqwestError)?;
  info!(%status, "openai upload image file response");

  classify_openai_http_error(status, Some(&text))?;

  let parsed: FileWire = serde_json::from_str(&text)
    .map_err(|e| OpenAiGenericApiError::SerdeResponseParseErrorWithBody(e, text.clone()))?;
  Ok(UploadedImageFile { file_id: parsed.id, bytes: parsed.bytes })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn empty_bytes_rejected() {
    let key = OpenAiApiKey::new("sk-test");
    let err = upload_image_file(UploadImageFileArgs {
      api_key: &key,
      request: UploadImageFileRequest::png(vec![], "a.png"),
    })
    .await
    .unwrap_err();
    assert!(matches!(err, OpenAiError::Client(OpenAiClientError::InvalidRequest(_))));
  }

  #[test]
  fn response_wire_parses_and_converts() {
    let wire: FileWire = serde_json::from_str(r#"{ "id": "file-abc123", "bytes": 5120, "purpose": "vision" }"#).unwrap();
    let uploaded = UploadedImageFile { file_id: wire.id, bytes: wire.bytes };
    assert_eq!(uploaded.clone().into_source(), InputImageSource::FileId("file-abc123".into()));
    assert_eq!(uploaded.bytes, 5120);
  }

  #[test]
  fn debug_elides_bytes() {
    let dbg = format!("{:?}", UploadImageFileRequest::png(vec![0; 4096], "a.png"));
    assert!(dbg.contains("len: 4096"), "{dbg}");
  }

  #[tokio::test]
  #[ignore] // live — needs external/credentials/openai_api_key.txt (uploads are free)
  async fn live_upload_image_file() -> anyhow::Result<()> {
    use crate::test_utils::get_test_api_key::get_test_api_key;
    use crate::test_utils::setup_test_logging::setup_test_logging;
    setup_test_logging();
    let key = get_test_api_key()?;
    // A 1×1 transparent PNG.
    let png: &[u8] = &[
      0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
      0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
      0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x62, 0x00, 0x01, 0x00, 0x00,
      0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
      0x42, 0x60, 0x82,
    ];
    let uploaded = upload_image_file(UploadImageFileArgs {
      api_key: &key,
      request: UploadImageFileRequest::png(png.to_vec(), "foundation-test.png"),
    })
    .await
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    anyhow::ensure!(uploaded.file_id.starts_with("file-"), "unexpected id: {}", uploaded.file_id);
    Ok(())
  }
}
