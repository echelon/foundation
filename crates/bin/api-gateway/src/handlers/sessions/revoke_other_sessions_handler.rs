//! `DELETE /v1/auth/sessions?others=true` — "log out everywhere else" (auth.md §3): bumps the
//! session-version clock, revokes every other session, and re-mints this cookie.

use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::response::Response;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::auth::service;
use crate::error::{ApiError, ApiResult};
use crate::handlers::api_types::OkResponse;
use crate::handlers::support::{req_meta, with_reminted_cookie};
use crate::middleware::AuthContext;
use crate::state::AppState;

#[derive(Debug, Deserialize, IntoParams)]
pub struct RevokeSessionsQuery {
  /// Must be `true` — guards against an accidental bare DELETE.
  #[serde(default)]
  pub others: bool,
}

#[utoipa::path(
  delete, path = "/v1/auth/sessions",
  tag = "sessions",
  security(("session_cookie" = [])),
  params(RevokeSessionsQuery),
  responses(
    (status = 200, description = "Other sessions revoked; this cookie re-minted", body = OkResponse),
    (status = 400, description = "`others=true` not passed")
  )
)]
pub async fn revoke_other_sessions_handler(
  State(state): State<AppState>,
  auth: AuthContext,
  headers: HeaderMap,
  Query(q): Query<RevokeSessionsQuery>,
) -> ApiResult<Response> {
  if !q.others {
    return Err(ApiError::BadRequest("pass ?others=true to log out other sessions".into()));
  }
  let meta = req_meta(&headers);
  let token =
    service::revoke_other_sessions(&state.pool, &state.jwt, &auth.user, &auth.session.id, meta.ip.as_deref()).await?;
  Ok(with_reminted_cookie(&state, &token, OkResponse::ok()))
}
