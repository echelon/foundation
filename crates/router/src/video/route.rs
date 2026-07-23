//! The built video route (mirrors `image::route`). Veo is ASYNC at the
//! provider: dispatch STARTS operations (one per slice) and the
//! google-gemini-polling-job drives them to completion.

use google_gemini_api_client::api::requests::videos::generate_gemini_omni_flash::generate_gemini_omni_flash::{
  generate_gemini_omni_flash, generate_gemini_omni_flash_wire_json, GeminiOmniFlashRequest,
  GenerateGeminiOmniFlashArgs,
};
use google_gemini_api_client::api::requests::videos::generate_veo_3_1::generate_veo_3_1::{
  generate_veo_3_1_wire_json, Veo31Request,
};
use google_gemini_api_client::api::requests::videos::generate_veo_3_1_fast::generate_veo_3_1_fast::{
  generate_veo_3_1_fast_wire_json, Veo31FastRequest,
};
use google_gemini_api_client::api::requests::videos::generate_veo_3_1_lite::generate_veo_3_1_lite::{
  generate_veo_3_1_lite_wire_json, Veo31LiteRequest,
};

use google_gemini_api_client::api::requests::videos::generate_veo_3_1::generate_veo_3_1::{
  generate_veo_3_1, GenerateVeo31Args,
};
use google_gemini_api_client::api::requests::videos::generate_veo_3_1_fast::generate_veo_3_1_fast::{
  generate_veo_3_1_fast, GenerateVeo31FastArgs,
};
use google_gemini_api_client::api::requests::videos::generate_veo_3_1_lite::generate_veo_3_1_lite::{
  generate_veo_3_1_lite, GenerateVeo31LiteArgs,
};
use google_gemini_api_client::api::types::image_types::image_input::ImageInput;
use google_gemini_api_client::api::types::video_types::veo_image_input::VeoImageInput;
use google_gemini_api_client::creds::google_gemini_api_key::GoogleGeminiApiKey;

use crate::error::RouterError;
use crate::image::adjustments::ParamAdjustment;
use crate::image::route::ProviderHandle;
use crate::image::route::quote_from_breakdowns;
use crate::input_strategy::{input_strategy_for, InputStrategy};
use crate::pricing::PriceQuote;
use crate::video::plan::RouterVideoModel;

/// A Google video route's slices + the pending upload-first i2v input url.
#[derive(Debug)]
pub struct GoogleVideoSlices<R> {
  pub requests: Vec<R>,
  pub input_url: Option<String>,
}

/// One Veo slice, typed — what the outbound dispatcher matches on to START the
/// provider operation.
#[derive(Debug)]
pub enum GoogleVideoSliceRef<'a> {
  Veo3_1(&'a Veo31Request),
  Veo3_1Fast(&'a Veo31FastRequest),
  Veo3_1Lite(&'a Veo31LiteRequest),
  OmniFlash(&'a GeminiOmniFlashRequest),
}

/// The built, ready-to-dispatch video route. One variant per catalog model.
#[derive(Debug)]
pub enum VideoRoute {
  GoogleVeo3_1(GoogleVideoSlices<Veo31Request>),
  GoogleVeo3_1Fast(GoogleVideoSlices<Veo31FastRequest>),
  GoogleVeo3_1Lite(GoogleVideoSlices<Veo31LiteRequest>),
  GoogleOmniFlash(GoogleVideoSlices<GeminiOmniFlashRequest>),
}

/// The i2v conditioning frame in dispatch-ready form. Which variant a route
/// wants is [`VideoRoute::prefers_files_api_input`]'s call: Veo's
/// predictLongRunning REJECTS Files-API `fileUri` refs (live-verified) so its
/// frame rides inline; Omni's Interactions API takes Files-API refs.
#[derive(Clone, Debug)]
pub enum RoutedVideoInput {
  Inline { mime: String, bytes: Vec<u8> },
  FileUri { uri: String, mime: String },
}

/// A build's result: the route plus every param adjustment made along the way.
#[derive(Debug)]
pub struct BuiltVideoRoute {
  pub route: VideoRoute,
  pub adjustments: Vec<ParamAdjustment>,
}

impl VideoRoute {
  pub fn model(&self) -> RouterVideoModel {
    match self {
      Self::GoogleVeo3_1(_) => RouterVideoModel::GoogleVeo3_1,
      Self::GoogleVeo3_1Fast(_) => RouterVideoModel::GoogleVeo3_1Fast,
      Self::GoogleVeo3_1Lite(_) => RouterVideoModel::GoogleVeo3_1Lite,
      Self::GoogleOmniFlash(_) => RouterVideoModel::GoogleOmniFlash,
    }
  }

  /// True when the i2v frame should be uploaded to Google's Files API and
  /// passed as a ref (Omni); false = inline base64 (Veo — fileUri rejected).
  pub fn prefers_files_api_input(&self) -> bool {
    matches!(self, Self::GoogleOmniFlash(_))
  }

  pub fn provider(&self) -> catalog::Provider {
    self.model().spec().provider
  }

  pub fn input_strategy(&self) -> InputStrategy {
    input_strategy_for(self.provider())
  }

  /// The pending upload-first i2v input url, if any.
  pub fn input_url(&self) -> Option<&str> {
    match self {
      Self::GoogleVeo3_1(s) => s.input_url.as_deref(),
      Self::GoogleVeo3_1Fast(s) => s.input_url.as_deref(),
      Self::GoogleVeo3_1Lite(s) => s.input_url.as_deref(),
      Self::GoogleOmniFlash(s) => s.input_url.as_deref(),
    }
  }

  /// How many provider operations this route fans out to (= the plan's `n`).
  pub fn fan_out(&self) -> usize {
    match self {
      Self::GoogleVeo3_1(s) => s.requests.len(),
      Self::GoogleVeo3_1Fast(s) => s.requests.len(),
      Self::GoogleVeo3_1Lite(s) => s.requests.len(),
      Self::GoogleOmniFlash(s) => s.requests.len(),
    }
  }

  /// Typed access to one slice — the outbound dispatcher starts its operation.
  pub fn google_slice(&self, index: usize) -> Option<GoogleVideoSliceRef<'_>> {
    match self {
      Self::GoogleVeo3_1(s) => s.requests.get(index).map(GoogleVideoSliceRef::Veo3_1),
      Self::GoogleVeo3_1Fast(s) => s.requests.get(index).map(GoogleVideoSliceRef::Veo3_1Fast),
      Self::GoogleVeo3_1Lite(s) => s.requests.get(index).map(GoogleVideoSliceRef::Veo3_1Lite),
      Self::GoogleOmniFlash(s) => s.requests.get(index).map(GoogleVideoSliceRef::OmniFlash),
    }
  }

  /// Billed seconds per video AFTER coercion (Veo bills per second; `None`
  /// duration means the server default, 8 s). All slices share one duration.
  pub fn seconds_per_video(&self) -> i64 {
    let duration = match self {
      Self::GoogleVeo3_1(s) => s.requests.first().and_then(|r| r.duration),
      Self::GoogleVeo3_1Fast(s) => s.requests.first().and_then(|r| r.duration),
      Self::GoogleVeo3_1Lite(s) => s.requests.first().and_then(|r| r.duration),
      // Omni's length is prompt-driven: the request carries only the ESTIMATE.
      Self::GoogleOmniFlash(s) => {
        return s.requests.first().and_then(|r| r.expected_output_seconds).unwrap_or(8);
      }
    };
    duration.map_or(8, |d| d.seconds())
  }

  /// Price the whole route, delegating every slice to the client estimator.
  /// Veo prices exactly per the params (per-second, blocked-not-billed).
  pub fn price(&self) -> PriceQuote {
    use client_core::CostCalculator;
    let breakdowns: Vec<client_core::CostBreakdown> = match self {
      Self::GoogleVeo3_1(s) => s.requests.iter().map(|r| r.estimate_cost()).collect(),
      Self::GoogleVeo3_1Fast(s) => s.requests.iter().map(|r| r.estimate_cost()).collect(),
      Self::GoogleVeo3_1Lite(s) => s.requests.iter().map(|r| r.estimate_cost()).collect(),
      Self::GoogleOmniFlash(s) => s.requests.iter().map(|r| r.estimate_cost()).collect(),
    };
    quote_from_breakdowns(&breakdowns, "video")
  }

  /// The absolute provider URL this route's requests post to — for outbound logs.
  pub fn endpoint_url(&self) -> String {
    match self {
      // Omni rides the Interactions API (background mode), not predictLongRunning.
      Self::GoogleOmniFlash(_) => "https://generativelanguage.googleapis.com/v1beta/interactions".to_string(),
      _ => format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:predictLongRunning",
        self.model().spec().provider_model_id
      ),
    }
  }

  /// The EXACT provider-native JSON body slice `index` would send.
  pub fn request_json(&self, index: usize) -> Result<serde_json::Value, RouterError> {
    let out_of_range = || RouterError::Invalid(format!("slice index {index} out of range"));
    match self {
      Self::GoogleVeo3_1(s) => generate_veo_3_1_wire_json(s.requests.get(index).ok_or_else(out_of_range)?),
      Self::GoogleVeo3_1Fast(s) => generate_veo_3_1_fast_wire_json(s.requests.get(index).ok_or_else(out_of_range)?),
      Self::GoogleVeo3_1Lite(s) => generate_veo_3_1_lite_wire_json(s.requests.get(index).ok_or_else(out_of_range)?),
      Self::GoogleOmniFlash(s) => {
        generate_gemini_omni_flash_wire_json(s.requests.get(index).ok_or_else(out_of_range)?)
      }
    }
    .map_err(|e| RouterError::Provider(e.to_string()))
  }

  /// START one slice's provider work (all google video models are async: the
  /// polling job lands results later). `input` is the dispatcher-prepared i2v
  /// frame in the form [`Self::prefers_files_api_input`] asked for.
  pub async fn start_google_slice(
    &self,
    api_key: &GoogleGeminiApiKey,
    index: usize,
    input: Option<RoutedVideoInput>,
  ) -> Result<ProviderHandle, RouterError> {
    let out_of_range = || RouterError::Invalid(format!("slice index {index} out of range"));
    let veo_input = |input: Option<RoutedVideoInput>| {
      input.map(|i| match i {
        RoutedVideoInput::Inline { mime, bytes } => VeoImageInput::InlineBytes { mime, bytes },
        RoutedVideoInput::FileUri { uri, mime } => VeoImageInput::FileUri { uri, mime },
      })
    };
    let handle = match self {
      Self::GoogleVeo3_1(s) => {
        let mut request = s.requests.get(index).ok_or_else(out_of_range)?.clone();
        request.image = veo_input(input);
        generate_veo_3_1(GenerateVeo31Args { api_key, request }).await.map(|op| op.0)
      }
      Self::GoogleVeo3_1Fast(s) => {
        let mut request = s.requests.get(index).ok_or_else(out_of_range)?.clone();
        request.image = veo_input(input);
        generate_veo_3_1_fast(GenerateVeo31FastArgs { api_key, request }).await.map(|op| op.0)
      }
      Self::GoogleVeo3_1Lite(s) => {
        let mut request = s.requests.get(index).ok_or_else(out_of_range)?.clone();
        request.image = veo_input(input);
        generate_veo_3_1_lite(GenerateVeo31LiteArgs { api_key, request }).await.map(|op| op.0)
      }
      Self::GoogleOmniFlash(s) => {
        let mut request = s.requests.get(index).ok_or_else(out_of_range)?.clone();
        request.images = input
          .map(|i| match i {
            RoutedVideoInput::Inline { mime, bytes } => ImageInput::InlineBytes { mime, bytes },
            RoutedVideoInput::FileUri { uri, mime } => ImageInput::FileUri { uri, mime },
          })
          .into_iter()
          .collect();
        generate_gemini_omni_flash(GenerateGeminiOmniFlashArgs { api_key, request }).await.map(|h| h.0)
      }
    }
    .map_err(|e| RouterError::Provider(e.to_string()))?;
    Ok(ProviderHandle { provider_job_id: handle })
  }
}
