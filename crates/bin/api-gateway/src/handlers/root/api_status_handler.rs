//! `GET /v1/api_status` — the frontend's "is it just me?" probe.
//!
//! Deliberately minimal and dependency-free: if this handler answers, the API is reachable —
//! it does NOT touch the database (that's `/health`), so a degraded DB never makes the probe
//! lie about reachability. `message` carries an operator-set status/downtime notice
//! (`API_STATUS_MESSAGE`, set at deploy) for frontends to show as a banner.

use axum::extract::State;
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiStatusResponse {
  /// True whenever the API answered — reachability is the signal.
  pub ok: bool,
  /// Operator-set status/downtime message; null when all is quiet.
  #[schema(example = "Scheduled maintenance 03:00–03:30 UTC — logins may be flaky.")]
  pub message: Option<String>,
}

#[utoipa::path(
  get, path = "/v1/api_status",
  tag = "meta",
  responses((status = 200, description = "The API is reachable (+ any operator status message)", body = ApiStatusResponse))
)]
pub async fn api_status_handler(State(state): State<AppState>) -> Json<ApiStatusResponse> {
  Json(ApiStatusResponse { ok: true, message: state.status_message.clone() })
}
