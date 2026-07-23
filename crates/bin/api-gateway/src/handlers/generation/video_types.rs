//! Wire types for the VIDEO generation endpoints (mirrors `types.rs` for images).
//! Familiar provider-style values ("1080p", "16:9", 8) map onto the router's enums.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use foundation_router::video::plan::{
  RouterVideoAspectRatio, RouterVideoDuration, RouterVideoResolution,
};
use ids::AccountId;

use super::types::{CostLineItem, CostResponse};
use crate::error::{ApiError, ApiResult};
use crate::generation::stored_params::StoredVideoParams;
use crate::state::AppState;

#[derive(Copy, Clone, Debug, Deserialize, Serialize, ToSchema)]
pub enum ApiVideoResolution {
  #[serde(rename = "720p")]
  #[schema(rename = "720p")]
  P720,
  #[serde(rename = "1080p")]
  #[schema(rename = "1080p")]
  P1080,
  #[serde(rename = "4k")]
  #[schema(rename = "4k")]
  K4,
}

impl From<ApiVideoResolution> for RouterVideoResolution {
  fn from(r: ApiVideoResolution) -> Self {
    match r {
      ApiVideoResolution::P720 => Self::P720,
      ApiVideoResolution::P1080 => Self::P1080,
      ApiVideoResolution::K4 => Self::K4,
    }
  }
}

impl From<RouterVideoResolution> for ApiVideoResolution {
  fn from(r: RouterVideoResolution) -> Self {
    match r {
      RouterVideoResolution::P720 => Self::P720,
      RouterVideoResolution::P1080 => Self::P1080,
      RouterVideoResolution::K4 => Self::K4,
    }
  }
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize, ToSchema)]
pub enum ApiVideoAspectRatio {
  #[serde(rename = "16:9")]
  #[schema(rename = "16:9")]
  Landscape,
  #[serde(rename = "9:16")]
  #[schema(rename = "9:16")]
  Portrait,
}

impl From<ApiVideoAspectRatio> for RouterVideoAspectRatio {
  fn from(a: ApiVideoAspectRatio) -> Self {
    match a {
      ApiVideoAspectRatio::Landscape => Self::Landscape16x9,
      ApiVideoAspectRatio::Portrait => Self::Portrait9x16,
    }
  }
}

impl From<RouterVideoAspectRatio> for ApiVideoAspectRatio {
  fn from(a: RouterVideoAspectRatio) -> Self {
    match a {
      RouterVideoAspectRatio::Landscape16x9 => Self::Landscape,
      RouterVideoAspectRatio::Portrait9x16 => Self::Portrait,
    }
  }
}

/// Wire duration is a plain number of seconds; only the provider-supported
/// lengths are valid (Veo: 4, 6, 8).
pub fn duration_from_seconds(seconds: u8) -> ApiResult<RouterVideoDuration> {
  match seconds {
    4 => Ok(RouterVideoDuration::S4),
    6 => Ok(RouterVideoDuration::S6),
    8 => Ok(RouterVideoDuration::S8),
    other => Err(ApiError::BadRequest(format!("duration_seconds must be 4, 6, or 8 — got {other}"))),
  }
}

fn default_n() -> u8 {
  1
}

/// The video generation params common to the Any and model-specific planes.
#[derive(Debug, Deserialize, ToSchema)]
pub struct VideoParamsBody {
  /// The account this job bills to (same contract as the image plane).
  #[schema(value_type = Option<String>, example = "acct_01k0…")]
  pub account_id: Option<AccountId>,
  #[schema(example = "A candle flame flickers in a dark room, slow dolly-in")]
  pub prompt: String,
  /// What NOT to generate.
  pub negative_prompt: Option<String>,
  /// Image-to-video conditioning frame: a `file_…` id (from `POST /v1/files`)
  /// or a public https URL.
  pub image_input: Option<String>,
  /// How many videos (1–10). One provider operation each; results arrive as
  /// each finishes (partial completion).
  #[serde(default = "default_n")]
  #[schema(minimum = 1, maximum = 10, default = 1)]
  pub n: u8,
  pub resolution: Option<ApiVideoResolution>,
  /// Clip length in seconds — 4, 6, or 8. 1080p/4K forces 8 (coerced and reported).
  #[schema(example = 8)]
  pub duration_seconds: Option<u8>,
  pub aspect_ratio: Option<ApiVideoAspectRatio>,
  /// Dry run: validate, price, and log the EXACT provider payload(s) — never send.
  #[serde(default)]
  pub dry_run: bool,
}

impl VideoParamsBody {
  /// Resolve the i2v ref + map enums into the persisted router param shape.
  pub async fn into_stored(
    self,
    state: &AppState,
    account_id: &AccountId,
    model: &str,
  ) -> ApiResult<StoredVideoParams> {
    let image_url = match &self.image_input {
      Some(r) => Some(crate::generation::service::resolve_one(state, account_id, r).await?),
      None => None,
    };
    let duration = self.duration_seconds.map(duration_from_seconds).transpose()?;
    Ok(StoredVideoParams {
      model: model.to_string(),
      prompt: self.prompt,
      negative_prompt: self.negative_prompt,
      image_url,
      n: self.n,
      resolution: self.resolution.map(Into::into),
      duration,
      aspect_ratio: self.aspect_ratio.map(Into::into),
    })
  }
}

/// Price a stored video param set without persisting anything (the cost twins).
pub fn video_cost_response(params: &StoredVideoParams) -> ApiResult<CostResponse> {
  let plan = params
    .to_plan()
    .ok_or_else(|| ApiError::BadRequest(format!("unknown or unrouted model: {}", params.model)))?;
  let built =
    foundation_router::video::build::build_video_route(plan).map_err(|e| ApiError::BadRequest(e.to_string()))?;
  let quote = built.route.price();
  Ok(CostResponse {
    certainty: match quote.certainty {
      foundation_router::pricing::PriceCertainty::Exact => "exact",
      foundation_router::pricing::PriceCertainty::Estimated => "estimated",
      foundation_router::pricing::PriceCertainty::PostBilled => "post_billed",
    }
    .to_string(),
    amount_micros: quote.amount_micros,
    max_micros: quote.max_micros,
    display: quote.amount_display(),
    line_items: quote
      .line_items
      .iter()
      .map(|li| CostLineItem { label: li.label.clone(), amount_micros: li.amount_micros })
      .collect(),
    param_adjustments: serde_json::to_value(&built.adjustments)
      .and_then(serde_json::from_value)
      .map_err(|e| ApiError::Internal(e.to_string()))?,
  })
}
