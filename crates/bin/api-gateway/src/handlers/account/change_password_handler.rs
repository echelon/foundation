//! `PUT /v1/auth/password` — change password with fresh re-auth (auth.md §3).

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use utoipa::ToSchema;

use crate::auth::service;
use crate::error::ApiResult;
use crate::handlers::api_types::OkResponse;
use crate::handlers::support::{req_meta, with_reminted_cookie};
use crate::middleware::AuthContext;
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct ChangePasswordBody {
  /// Required when a password exists. SSO-only accounts (see `/me`'s `has_password: false`)
  /// omit it to SET their first password.
  pub current_password: Option<String>,
  pub new_password: String,
  /// Also revoke every OTHER session (bumps the session-version clock; this cookie re-mints).
  #[serde(default)]
  pub logout_other_sessions: bool,
}

/// Change the password — or, for SSO-only accounts (no password yet), set the first one
/// without a `current_password`.
#[utoipa::path(
  put, path = "/v1/auth/password",
  tag = "account",
  security(("session_cookie" = [])),
  request_body = ChangePasswordBody,
  responses(
    (status = 200, description = "Password changed/set (cookie re-minted when other sessions were purged)", body = OkResponse),
    (status = 400, description = "New password too short, or current password missing when one exists"),
    (status = 401, description = "Current password incorrect")
  )
)]
pub async fn change_password_handler(
  State(state): State<AppState>,
  auth: AuthContext,
  headers: HeaderMap,
  Json(body): Json<ChangePasswordBody>,
) -> ApiResult<Response> {
  let meta = req_meta(&headers);
  let reminted = service::change_password(
    &state.pool,
    &state.jwt,
    &auth.user,
    service::ChangePasswordArgs {
      current_password: body.current_password.as_deref(),
      new_password: &body.new_password,
      logout_others: body.logout_other_sessions,
      current_session_id: &auth.session.id,
      ip: meta.ip.as_deref(),
    },
  )
  .await?;

  Ok(match reminted.new_token {
    Some(token) => with_reminted_cookie(&state, &token, OkResponse::ok()),
    None => (StatusCode::OK, Json(OkResponse::ok())).into_response(),
  })
}
