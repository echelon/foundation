//! `DELETE /v1/auth/login_methods/{sso_id}` — disconnect an SSO connection.

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;

use ids::SsoConnectionId;

use crate::auth::service;
use crate::error::{ApiError, ApiResult};
use crate::handlers::api_types::OkResponse;
use crate::handlers::support::req_meta;
use crate::middleware::AuthContext;
use crate::state::AppState;

/// Refused (403) when the user has neither a password nor a verified email — removing the
/// connection could lock them out with no way back in (removal rule per bt, 2026-07-21).
#[utoipa::path(
  delete, path = "/v1/auth/login_methods/{sso_id}",
  tag = "account",
  security(("session_cookie" = [])),
  params(("sso_id" = String, Path, description = "SSO connection id (`sso_…`)")),
  responses(
    (status = 200, description = "Disconnected", body = OkResponse),
    (status = 400, description = "Invalid id"),
    (status = 403, description = "No password and no verified email — removal could lock the user out"),
    (status = 404, description = "Unknown or foreign connection id")
  )
)]
pub async fn unlink_sso_handler(
  State(state): State<AppState>,
  auth: AuthContext,
  headers: HeaderMap,
  Path(sso_id): Path<String>,
) -> ApiResult<Json<OkResponse>> {
  let sso_id: SsoConnectionId = sso_id.parse().map_err(|_| ApiError::BadRequest("invalid connection id".into()))?;
  let meta = req_meta(&headers);
  service::unlink_sso(&state.pool, &auth.user, &sso_id, meta.ip.as_deref()).await?;
  Ok(Json(OkResponse::ok()))
}
