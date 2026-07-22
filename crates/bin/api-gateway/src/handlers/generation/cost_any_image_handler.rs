//! `POST /v1/any/image/cost` — the Any plane's cost twin (api_any.md): identical body,
//! prices without committing anything.

use axum::extract::State;
use axum::Json;

use super::generate_any_image_handler::AnyImageBody;
use super::types::{cost_response, CostResponse};
use crate::error::ApiResult;
use crate::middleware::AuthContext;
use crate::state::AppState;

/// Quote the price (USD micros) of the EXACT request you would send to `POST /v1/any/image`,
/// including the param adjustments the router would make. Nothing is persisted or billed.
#[utoipa::path(
  post, path = "/v1/any/image/cost",
  tag = "generation",
  security(("session_cookie" = [])),
  request_body = AnyImageBody,
  responses(
    (status = 200, description = "The quote", body = CostResponse),
    (status = 400, description = "Unknown model or invalid params")
  )
)]
pub async fn cost_any_image_handler(
  State(state): State<AppState>,
  auth: AuthContext,
  Json(body): Json<AnyImageBody>,
) -> ApiResult<Json<CostResponse>> {
  let _ = &auth; // any signed-in user may price a request
  let stored = body.params.into_stored(&state, &body.model).await?;
  Ok(Json(cost_response(&stored)?))
}
