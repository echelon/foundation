//! `POST /v1/any/video/cost` — the video Any plane's cost twin: identical body,
//! prices without committing anything.

use axum::extract::State;
use axum::Json;

use super::generate_any_video_handler::AnyVideoBody;
use super::types::{resolve_caller_account, CostResponse};
use super::video_types::video_cost_response;
use crate::error::ApiResult;
use crate::middleware::Caller;
use crate::state::AppState;

/// Quote the price (USD micros) of the EXACT request you would send to `POST /v1/any/video`,
/// including the param adjustments the router would make. Nothing is persisted or billed.
#[utoipa::path(
  post, path = "/v1/any/video/cost",
  tag = "generation",
  security(("session_cookie" = []), ("api_key" = [])),
  request_body = AnyVideoBody,
  responses(
    (status = 200, description = "The quote", body = CostResponse),
    (status = 400, description = "Unknown model or invalid params")
  )
)]
pub async fn cost_any_video_handler(
  State(state): State<AppState>,
  caller: Caller,
  Json(body): Json<AnyVideoBody>,
) -> ApiResult<Json<CostResponse>> {
  let account_id = resolve_caller_account(&state, &caller, body.params.account_id.as_ref()).await?;
  let stored = body.params.into_stored(&state, &account_id, &body.model).await?;
  Ok(Json(video_cost_response(&stored)?))
}
