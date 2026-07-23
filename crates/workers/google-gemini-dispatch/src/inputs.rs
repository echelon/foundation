//! Upload-first inputs: Gemini cannot fetch public URLs, so before a call we
//! download each of OUR CDN input urls and re-upload the bytes to Google's
//! Files API (free, ~48 h retention), then inject the returned refs.

use google_gemini_api_client::api::requests::files::upload_image_file::upload_image_file::{
  upload_image_file, UploadImageFileArgs, UploadImageFileRequest, UploadedFileRef,
};
use google_gemini_api_client::api::types::image_types::image_input::ImageInput;
use foundation_router::video::route::RoutedVideoInput;

use crate::context::DispatchContext;

/// Fetch one CDN url and push its bytes to the Google Files API.
async fn upload_one(ctx: &DispatchContext, url: &str) -> Result<UploadedFileRef, String> {
  let response = ctx.http.get(url).send().await.map_err(|e| format!("input fetch failed for {url}: {e}"))?;
  if !response.status().is_success() {
    return Err(format!("input fetch failed for {url}: HTTP {}", response.status()));
  }
  let mime = response
    .headers()
    .get(reqwest::header::CONTENT_TYPE)
    .and_then(|v| v.to_str().ok())
    .unwrap_or("image/png")
    .to_string();
  let bytes = response.bytes().await.map_err(|e| format!("input read failed for {url}: {e}"))?.to_vec();
  upload_image_file(UploadImageFileArgs {
    api_key: &ctx.api_key,
    request: UploadImageFileRequest { bytes, mime, display_name: None },
  })
  .await
  .map_err(|e| format!("Google Files upload failed for {url}: {e}"))
}

/// Image-edit inputs: every CDN url → a `file_data` ref, order preserved.
pub async fn upload_image_inputs(ctx: &DispatchContext, urls: &[String]) -> Result<Vec<ImageInput>, String> {
  let mut inputs = Vec::with_capacity(urls.len());
  for url in urls {
    let file = upload_one(ctx, url).await?;
    inputs.push(ImageInput::FileUri { uri: file.uri, mime: file.mime_type });
  }
  Ok(inputs)
}

/// The i2v conditioning frame, when the job has one, in the form the route
/// asked for. Veo's predictLongRunning REJECTS Files-API `fileUri` refs
/// ("`fileUri` isn't supported by this model", live-verified) so its frame
/// rides inline; Omni's Interactions API takes (and prefers) Files-API refs.
pub async fn prepare_video_input(
  ctx: &DispatchContext,
  url: Option<&str>,
  use_files_api: bool,
) -> Result<Option<RoutedVideoInput>, String> {
  let Some(url) = url else { return Ok(None) };
  if use_files_api {
    let file = upload_one(ctx, url).await?;
    return Ok(Some(RoutedVideoInput::FileUri { uri: file.uri, mime: file.mime_type }));
  }
  let response = ctx.http.get(url).send().await.map_err(|e| format!("input fetch failed for {url}: {e}"))?;
  if !response.status().is_success() {
    return Err(format!("input fetch failed for {url}: HTTP {}", response.status()));
  }
  let mime = response
    .headers()
    .get(reqwest::header::CONTENT_TYPE)
    .and_then(|v| v.to_str().ok())
    .unwrap_or("image/png")
    .to_string();
  let bytes = response.bytes().await.map_err(|e| format!("input read failed for {url}: {e}"))?.to_vec();
  Ok(Some(RoutedVideoInput::Inline { mime, bytes }))
}
