//! Shared request/response types for the generation endpoints. The wire vocabulary uses
//! familiar provider-style values ("1024x1024", "png") and maps onto the router's enums.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use enums::{ApiShape, JobSource};
use foundation_router::image::plan::{
  RouterImageQuality, RouterImageSize, RouterInputFidelity, RouterOutputFormat,
};

use crate::error::{ApiError, ApiResult};
use crate::generation::service::{resolve_image_refs, EnqueueContext};
use crate::generation::stored_params::StoredImageParams;
use crate::middleware::Caller;
use crate::state::AppState;
use ids::AccountId;

#[derive(Copy, Clone, Debug, Deserialize, Serialize, ToSchema)]
pub enum ApiImageSize {
  #[serde(rename = "1024x1024")]
  #[schema(rename = "1024x1024")]
  Square,
  #[serde(rename = "1536x1024")]
  #[schema(rename = "1536x1024")]
  Landscape,
  #[serde(rename = "1024x1536")]
  #[schema(rename = "1024x1536")]
  Portrait,
  #[serde(rename = "auto")]
  #[schema(rename = "auto")]
  Auto,
}

impl From<ApiImageSize> for RouterImageSize {
  fn from(s: ApiImageSize) -> Self {
    match s {
      ApiImageSize::Square => Self::Square1024,
      ApiImageSize::Landscape => Self::Landscape1536x1024,
      ApiImageSize::Portrait => Self::Portrait1024x1536,
      ApiImageSize::Auto => Self::Auto,
    }
  }
}

impl From<RouterImageSize> for ApiImageSize {
  fn from(s: RouterImageSize) -> Self {
    match s {
      RouterImageSize::Square1024 => Self::Square,
      RouterImageSize::Landscape1536x1024 => Self::Landscape,
      RouterImageSize::Portrait1024x1536 => Self::Portrait,
      RouterImageSize::Auto => Self::Auto,
    }
  }
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ApiImageQuality {
  Low,
  Medium,
  High,
  Auto,
}

impl From<ApiImageQuality> for RouterImageQuality {
  fn from(q: ApiImageQuality) -> Self {
    match q {
      ApiImageQuality::Low => Self::Low,
      ApiImageQuality::Medium => Self::Medium,
      ApiImageQuality::High => Self::High,
      ApiImageQuality::Auto => Self::Auto,
    }
  }
}

impl From<RouterImageQuality> for ApiImageQuality {
  fn from(q: RouterImageQuality) -> Self {
    match q {
      RouterImageQuality::Low => Self::Low,
      RouterImageQuality::Medium => Self::Medium,
      RouterImageQuality::High => Self::High,
      RouterImageQuality::Auto => Self::Auto,
    }
  }
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ApiOutputFormat {
  Png,
  Jpeg,
  Webp,
}

impl From<ApiOutputFormat> for RouterOutputFormat {
  fn from(f: ApiOutputFormat) -> Self {
    match f {
      ApiOutputFormat::Png => Self::Png,
      ApiOutputFormat::Jpeg => Self::Jpeg,
      ApiOutputFormat::Webp => Self::Webp,
    }
  }
}

impl From<RouterOutputFormat> for ApiOutputFormat {
  fn from(f: RouterOutputFormat) -> Self {
    match f {
      RouterOutputFormat::Png => Self::Png,
      RouterOutputFormat::Jpeg => Self::Jpeg,
      RouterOutputFormat::Webp => Self::Webp,
    }
  }
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ApiInputFidelity {
  Low,
  High,
}

impl From<ApiInputFidelity> for RouterInputFidelity {
  fn from(f: ApiInputFidelity) -> Self {
    match f {
      ApiInputFidelity::Low => Self::Low,
      ApiInputFidelity::High => Self::High,
    }
  }
}

impl From<RouterInputFidelity> for ApiInputFidelity {
  fn from(f: RouterInputFidelity) -> Self {
    match f {
      RouterInputFidelity::Low => Self::Low,
      RouterInputFidelity::High => Self::High,
    }
  }
}

fn default_n() -> u8 {
  1
}

/// The image generation/edit params common to the Any and model-specific planes.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ImageParamsBody {
  /// The account this job bills to. Required for session callers (who may belong to
  /// several accounts); optional for API-key callers — the key already names the account
  /// (and a mismatched value is rejected).
  #[schema(value_type = Option<String>, example = "acct_01k0…")]
  pub account_id: Option<AccountId>,
  #[schema(example = "A watercolor lighthouse at dusk")]
  pub prompt: String,
  /// Edit inputs: `file_…` ids (from `POST /v1/files`) or public https URLs.
  #[serde(default)]
  pub image_inputs: Vec<String>,
  /// Inpainting mask (transparent regions get regenerated) — `file_…` id or https URL.
  #[serde(default)]
  pub mask: Option<String>,
  /// How many images (1–10). Each is generated independently; results arrive as each
  /// finishes (partial completion).
  #[serde(default = "default_n")]
  #[schema(minimum = 1, maximum = 10, default = 1)]
  pub n: u8,
  pub size: Option<ApiImageSize>,
  pub quality: Option<ApiImageQuality>,
  pub output_format: Option<ApiOutputFormat>,
  /// Edit-input faithfulness — pruned (and reported) on models without it.
  pub input_fidelity: Option<ApiInputFidelity>,
  /// Dry run: validate, price, and log the EXACT provider payload(s) — but never send.
  /// The job is created terminal (`canceled`, failure_code `dry_run`) and costs nothing.
  #[serde(default)]
  pub dry_run: bool,
}

impl ImageParamsBody {
  /// Resolve refs + map enums into the persisted router param shape. `account_id` is the
  /// caller-resolved scope (see [`resolve_caller_account`]) — file refs resolve within it.
  pub async fn into_stored(
    self,
    state: &AppState,
    account_id: &AccountId,
    model: &str,
  ) -> ApiResult<StoredImageParams> {
    let image_urls = resolve_image_refs(state, account_id, &self.image_inputs).await?;
    let mask_url = match &self.mask {
      Some(m) => Some(crate::generation::service::resolve_one(state, account_id, m).await?),
      None => None,
    };
    Ok(StoredImageParams {
      model: model.to_string(),
      prompt: self.prompt,
      image_urls,
      mask_url,
      n: self.n,
      size: self.size.map(Into::into),
      quality: self.quality.map(Into::into),
      output_format: self.output_format.map(Into::into),
      input_fidelity: self.input_fidelity.map(Into::into),
    })
  }
}

/// Resolve which account the call is scoped to, and gate the caller onto it. Session
/// callers must NAME an account (they may belong to several) and be a member of it;
/// API-key callers ARE the account — naming a different one is rejected the same way an
/// unknown account is (404, no existence oracle).
pub async fn resolve_caller_account(
  state: &AppState,
  caller: &Caller,
  requested: Option<&AccountId>,
) -> ApiResult<AccountId> {
  match caller {
    Caller::Session(auth) => {
      let account_id =
        requested.ok_or_else(|| ApiError::BadRequest("account_id is required".into()))?;
      db_queries::members::find_membership(&state.pool, account_id, &auth.user.id)
        .await?
        .ok_or_else(|| ApiError::NotFound("account not found".into()))?;
      Ok(account_id.clone())
    }
    Caller::ApiKey(key) => {
      if requested.is_some_and(|a| *a != key.account_id) {
        return Err(ApiError::NotFound("account not found".into()));
      }
      Ok(key.account_id.clone())
    }
  }
}

/// The enqueue context for an already-gated caller: the door decides `source` and
/// attribution (`user_id` for sessions, `api_key_id` for keys) — jobs.md's provenance
/// columns.
pub fn enqueue_context<'a>(
  caller: &'a Caller,
  account_id: &'a AccountId,
  api_shape: ApiShape,
  dry_run: bool,
) -> EnqueueContext<'a> {
  match caller {
    Caller::Session(auth) => EnqueueContext {
      account_id,
      user_id: Some(&auth.user.id),
      api_key_id: None,
      source: JobSource::Dashboard,
      api_shape,
      dry_run,
    },
    Caller::ApiKey(key) => EnqueueContext {
      account_id,
      user_id: None,
      api_key_id: Some(&key.id),
      source: JobSource::ApiKey,
      api_shape,
      dry_run,
    },
  }
}

/// A price quote (cost twins + echoed on enqueue). All amounts are USD micros.
#[derive(Debug, Serialize, ToSchema)]
pub struct CostResponse {
  /// `exact`, `estimated` (good-faith; provider usage is authoritative), or `post_billed`.
  pub certainty: String,
  pub amount_micros: i64,
  /// Upper bound when the price is a range.
  pub max_micros: Option<i64>,
  /// Human-readable dollars, e.g. `"$0.1200"`.
  pub display: String,
  pub line_items: Vec<CostLineItem>,
  /// Params the router pruned/coerced to fit the model.
  pub param_adjustments: Vec<crate::generation::job_response::ParamAdjustmentBody>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CostLineItem {
  pub label: String,
  pub amount_micros: i64,
}

/// Price a stored-param set without persisting anything (the cost twins).
pub fn cost_response(params: &StoredImageParams) -> ApiResult<CostResponse> {
  let plan = params
    .to_plan()
    .ok_or_else(|| ApiError::BadRequest(format!("unknown or unrouted model: {}", params.model)))?;
  let built =
    foundation_router::image::build::build_image_route(plan).map_err(|e| ApiError::BadRequest(e.to_string()))?;
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
