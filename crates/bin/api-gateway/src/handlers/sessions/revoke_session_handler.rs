//! `DELETE /v1/auth/sessions/{id}` — terminate one session (auth.md §3).

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;

use crate::auth::service;
use crate::error::{ApiError, ApiResult};
use crate::handlers::api_types::OkResponse;
use crate::handlers::support::req_meta;
use crate::middleware::AuthContext;
use crate::state::AppState;

#[utoipa::path(
  delete, path = "/v1/auth/sessions/{id}",
  tag = "sessions",
  security(("session_cookie" = [])),
  params(("id" = String, Path, description = "Session id (`sess_…`)")),
  responses(
    (status = 200, description = "Session revoked", body = OkResponse),
    (status = 404, description = "Not your session / unknown id")
  )
)]
pub async fn revoke_session_handler(
  State(state): State<AppState>,
  auth: AuthContext,
  headers: HeaderMap,
  Path(id): Path<String>,
) -> ApiResult<Json<OkResponse>> {
  let session_id: ids::SessionId = id.parse().map_err(|_| ApiError::BadRequest("invalid session id".into()))?;
  let meta = req_meta(&headers);
  service::revoke_session(&state.pool, &auth.user, &session_id, meta.ip.as_deref()).await?;
  Ok(Json(OkResponse::ok()))
}
