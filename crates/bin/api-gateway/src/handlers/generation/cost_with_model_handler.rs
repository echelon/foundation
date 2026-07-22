//! `POST /v1/models/{slug}/generate/cost` — cost twin of the unified plane: identical
//! body, prices without committing anything.

use axum::extract::{Path, State};
use axum::Json;

use super::types::{cost_response, resolve_caller_account, CostResponse, ImageParamsBody};
use crate::error::{ApiError, ApiResult};
use crate::middleware::Caller;
use crate::state::AppState;

/// Quote the price (USD micros) of the EXACT request you would send to
/// `POST /v1/models/{slug}/generate`. Nothing is persisted or billed.
#[utoipa::path(
  post, path = "/v1/models/{slug}/generate/cost",
  tag = "generation",
  security(("session_cookie" = []), ("api_key" = [])),
  params(("slug" = String, Path, description = "Catalog model slug")),
  request_body = ImageParamsBody,
  responses(
    (status = 200, description = "The quote", body = CostResponse),
    (status = 400, description = "Invalid params"),
    (status = 404, description = "Unknown/unrouted model slug")
  )
)]
pub async fn cost_with_model_handler(
  State(state): State<AppState>,
  caller: Caller,
  Path(slug): Path<String>,
  Json(body): Json<ImageParamsBody>,
) -> ApiResult<Json<CostResponse>> {
  if catalog::by_slug(&slug).is_none() {
    return Err(ApiError::NotFound(format!("unknown model: {slug}")));
  }
  let account_id = resolve_caller_account(&state, &caller, body.account_id.as_ref()).await?;
  let stored = body.into_stored(&state, &account_id, &slug).await?;
  Ok(Json(cost_response(&stored)?))
}
