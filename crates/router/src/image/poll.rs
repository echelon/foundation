//! Slice polling: fetch one provider handle's state, normalized to a router-level shape
//! (the dispatcher persists outcomes; the router never loops).

use openai_client::api::requests::responses::get_response::{get_response, GetResponseArgs};
use openai_client::api::types::response_types::response_id::ResponseId;
use openai_client::api::types::response_types::response_status::ResponseStatus;
use openai_client::creds::openai_api_key::OpenAiApiKey;

use crate::error::RouterError;

#[derive(Debug)]
pub struct PollImageArgs<'a> {
  pub api_key: &'a OpenAiApiKey,
  /// The slice's provider handle (OpenAI `resp_…`).
  pub provider_job_id: &'a str,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RoutedSliceStatus {
  Pending,
  Succeeded,
  Failed,
}

pub struct RoutedImage {
  pub bytes: Vec<u8>,
  pub mime: &'static str,
  /// Bucket-path extension (with the dot), e.g. `.png`.
  pub extension: &'static str,
}

impl std::fmt::Debug for RoutedImage {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("RoutedImage").field("len", &self.bytes.len()).field("mime", &self.mime).finish()
  }
}

#[derive(Debug)]
pub struct RoutedImagePoll {
  pub status: RoutedSliceStatus,
  /// Present on success (usually exactly one image per slice).
  pub images: Vec<RoutedImage>,
  pub failure_message: Option<String>,
}

pub async fn poll_openai_image(args: PollImageArgs<'_>) -> Result<RoutedImagePoll, RouterError> {
  let response_id = ResponseId(args.provider_job_id.to_string());
  let snapshot = get_response(GetResponseArgs { api_key: args.api_key, response_id: &response_id })
    .await
    .map_err(|e| RouterError::Provider(e.to_string()))?;

  let status = match snapshot.status {
    ResponseStatus::Queued | ResponseStatus::InProgress => RoutedSliceStatus::Pending,
    ResponseStatus::Completed if !snapshot.images.is_empty() => RoutedSliceStatus::Succeeded,
    // Completed with no image = the tool produced nothing usable (e.g. refusal).
    _ => RoutedSliceStatus::Failed,
  };
  let images = snapshot
    .images
    .into_iter()
    .map(|img| {
      use openai_client::api::types::image_types::output_format::OutputFormat;
      let (mime, extension) = match img.format {
        OutputFormat::Png => ("image/png", ".png"),
        OutputFormat::Jpeg => ("image/jpeg", ".jpg"),
        OutputFormat::Webp => ("image/webp", ".webp"),
      };
      RoutedImage { bytes: img.bytes, mime, extension }
    })
    .collect();
  Ok(RoutedImagePoll {
    status,
    images,
    failure_message: snapshot.failure.map(|f| f.message),
  })
}
