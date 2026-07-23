//! Video slice polling: fetch one provider handle's state, normalized to the
//! router-level shape (mirrors `image::poll` — the polling job persists
//! outcomes; the router never loops). Two provider surfaces hide behind one
//! fn: Veo handles are operation names (`models/…/operations/…`, polled via
//! the operations API), Omni handles are interaction ids (polled via the
//! Interactions API).

use google_gemini_api_client::api::requests::files::download_file::download_file::{
  download_file, DownloadFileArgs,
};
use google_gemini_api_client::api::requests::files::download_video::download_video::{
  download_video, DownloadVideoArgs,
};
use google_gemini_api_client::api::requests::interactions::get_interaction::get_interaction::{
  get_interaction, GetInteractionArgs, InteractionPollOutcome,
};
use google_gemini_api_client::api::requests::videos::generate_gemini_omni_flash::generate_gemini_omni_flash::{
  omni_video_success_from_interaction, OmniVideoOutput,
};
use google_gemini_api_client::api::requests::videos::poll_video_operation::poll_video_operation::{
  poll_video_operation, PollVideoOperationArgs, VeoPollOutcome,
};
use google_gemini_api_client::api::requests::videos::VeoOperationName;
use google_gemini_api_client::api::requests::interactions::InteractionHandle;
use google_gemini_api_client::creds::google_gemini_api_key::GoogleGeminiApiKey;

use crate::error::RouterError;
use crate::image::poll::RoutedSliceStatus;
use crate::video::plan::RouterVideoModel;

#[derive(Debug)]
pub struct PollVideoArgs<'a> {
  pub api_key: &'a GoogleGeminiApiKey,
  /// Which model the handle belongs to — picks the provider poll surface.
  pub model: RouterVideoModel,
  /// The slice's provider handle (Veo operation name or Omni interaction id).
  pub provider_job_id: &'a str,
}

/// One finished video, in whatever form the provider handed it back.
#[derive(Clone)]
pub enum RoutedVideoPayload {
  /// Veo download URI (expires ~2 days) — fetch promptly.
  VeoUri(String),
  /// Omni Files-API ref (URI delivery) — fetch promptly.
  OmniFileRef(String),
  /// Omni inline delivery (small results ≤ 4 MB), already decoded.
  Inline { mime: String, bytes: Vec<u8> },
}

impl std::fmt::Debug for RoutedVideoPayload {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::VeoUri(u) => f.debug_tuple("VeoUri").field(u).finish(),
      Self::OmniFileRef(u) => f.debug_tuple("OmniFileRef").field(u).finish(),
      Self::Inline { mime, bytes } => {
        f.debug_struct("Inline").field("mime", mime).field("len", &bytes.len()).finish()
      }
    }
  }
}

#[derive(Debug)]
pub struct RoutedVideoPoll {
  pub status: RoutedSliceStatus,
  /// Present on success — fetch each with [`fetch_video_payload`].
  pub videos: Vec<RoutedVideoPayload>,
  pub failure_message: Option<String>,
}

pub async fn poll_google_video(args: PollVideoArgs<'_>) -> Result<RoutedVideoPoll, RouterError> {
  match args.model {
    RouterVideoModel::GoogleOmniFlash => poll_omni(args).await,
    _ => poll_veo(args).await,
  }
}

async fn poll_veo(args: PollVideoArgs<'_>) -> Result<RoutedVideoPoll, RouterError> {
  let operation = VeoOperationName(args.provider_job_id.to_string());
  let outcome = poll_video_operation(PollVideoOperationArgs { api_key: args.api_key, operation: &operation })
    .await
    .map_err(|e| RouterError::Provider(e.to_string()))?;
  Ok(match outcome {
    VeoPollOutcome::Pending => pending(),
    VeoPollOutcome::Succeeded(videos) => RoutedVideoPoll {
      status: RoutedSliceStatus::Succeeded,
      videos: videos.into_iter().map(|v| RoutedVideoPayload::VeoUri(v.uri)).collect(),
      failure_message: None,
    },
    VeoPollOutcome::Failed { code, message } => failed(match code {
      Some(code) => format!("provider error {code}: {message}"),
      None => message,
    }),
  })
}

async fn poll_omni(args: PollVideoArgs<'_>) -> Result<RoutedVideoPoll, RouterError> {
  let handle = InteractionHandle(args.provider_job_id.to_string());
  let outcome = get_interaction(GetInteractionArgs { api_key: args.api_key, interaction: &handle })
    .await
    .map_err(|e| RouterError::Provider(e.to_string()))?;
  Ok(match outcome {
    InteractionPollOutcome::InProgress => pending(),
    InteractionPollOutcome::Cancelled => failed("interaction was cancelled at the provider".to_string()),
    InteractionPollOutcome::Failed { message } => failed(message),
    InteractionPollOutcome::Completed(success) => {
      let video = omni_video_success_from_interaction(success).map_err(|e| RouterError::Provider(e.to_string()))?;
      RoutedVideoPoll {
        status: RoutedSliceStatus::Succeeded,
        videos: video
          .videos
          .into_iter()
          .map(|v| match v {
            OmniVideoOutput::Inline { mime, bytes } => RoutedVideoPayload::Inline { mime, bytes },
            OmniVideoOutput::FileUri(uri) => RoutedVideoPayload::OmniFileRef(uri),
          })
          .collect(),
        failure_message: None,
      }
    }
  })
}

fn pending() -> RoutedVideoPoll {
  RoutedVideoPoll { status: RoutedSliceStatus::Pending, videos: vec![], failure_message: None }
}

fn failed(message: String) -> RoutedVideoPoll {
  RoutedVideoPoll { status: RoutedSliceStatus::Failed, videos: vec![], failure_message: Some(message) }
}

/// Resolve one payload to `(mime, bytes)`, downloading when it's a ref.
pub async fn fetch_video_payload(
  api_key: &GoogleGeminiApiKey,
  payload: RoutedVideoPayload,
) -> Result<(String, Vec<u8>), RouterError> {
  match payload {
    RoutedVideoPayload::Inline { mime, bytes } => Ok((mime, bytes)),
    RoutedVideoPayload::VeoUri(uri) => {
      let bytes =
        download_video(DownloadVideoArgs { api_key, uri: &uri }).await.map_err(|e| RouterError::Provider(e.to_string()))?;
      Ok(("video/mp4".to_string(), bytes))
    }
    RoutedVideoPayload::OmniFileRef(name) => {
      let bytes =
        download_file(DownloadFileArgs { api_key, name: &name }).await.map_err(|e| RouterError::Provider(e.to_string()))?;
      Ok(("video/mp4".to_string(), bytes))
    }
  }
}
