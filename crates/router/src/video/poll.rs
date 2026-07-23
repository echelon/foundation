//! Video slice polling: fetch one Veo operation's state, normalized to the
//! router-level shape (mirrors `image::poll` — the polling job persists
//! outcomes; the router never loops).

use google_gemini_api_client::api::requests::files::download_video::download_video::{
  download_video, DownloadVideoArgs,
};
use google_gemini_api_client::api::requests::videos::VeoOperationName;
use google_gemini_api_client::api::requests::videos::poll_video_operation::poll_video_operation::{
  poll_video_operation, PollVideoOperationArgs, VeoPollOutcome,
};
use google_gemini_api_client::creds::google_gemini_api_key::GoogleGeminiApiKey;

use crate::error::RouterError;
use crate::image::poll::RoutedSliceStatus;

#[derive(Debug)]
pub struct PollVideoArgs<'a> {
  pub api_key: &'a GoogleGeminiApiKey,
  /// The slice's provider handle (a Veo operation name, `models/…/operations/…`).
  pub provider_job_id: &'a str,
}

#[derive(Debug)]
pub struct RoutedVideoPoll {
  pub status: RoutedSliceStatus,
  /// Present on success: provider download URIs (expire ~2 days) — fetch with
  /// [`download_routed_video`] promptly.
  pub video_uris: Vec<String>,
  pub failure_message: Option<String>,
}

pub async fn poll_google_video(args: PollVideoArgs<'_>) -> Result<RoutedVideoPoll, RouterError> {
  let operation = VeoOperationName(args.provider_job_id.to_string());
  let outcome = poll_video_operation(PollVideoOperationArgs { api_key: args.api_key, operation: &operation })
    .await
    .map_err(|e| RouterError::Provider(e.to_string()))?;
  Ok(match outcome {
    VeoPollOutcome::Pending => {
      RoutedVideoPoll { status: RoutedSliceStatus::Pending, video_uris: vec![], failure_message: None }
    }
    VeoPollOutcome::Succeeded(videos) => RoutedVideoPoll {
      status: RoutedSliceStatus::Succeeded,
      video_uris: videos.into_iter().map(|v| v.uri).collect(),
      failure_message: None,
    },
    VeoPollOutcome::Failed { code, message } => RoutedVideoPoll {
      status: RoutedSliceStatus::Failed,
      video_uris: vec![],
      failure_message: Some(match code {
        Some(code) => format!("provider error {code}: {message}"),
        None => message,
      }),
    },
  })
}

/// Fetch a succeeded operation's video bytes (MP4) from its download URI.
pub async fn download_routed_video(api_key: &GoogleGeminiApiKey, uri: &str) -> Result<Vec<u8>, RouterError> {
  download_video(DownloadVideoArgs { api_key, uri }).await.map_err(|e| RouterError::Provider(e.to_string()))
}
