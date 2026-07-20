//! Account/security handlers (accounts.md §2/§6, auth.md §3): change password, change email,
//! forgot/reset password, and session listing/termination. Thin wrappers over `auth::service`.

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::cookie;
use crate::auth::service;
use crate::auth::token::SESSION_TTL;
use crate::error::{ApiError, ApiResult};
use crate::handlers::auth_handlers::{req_meta, set_cookie_header};
use crate::mailer;
use crate::middleware::AuthContext;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ChangePasswordBody {
  pub current_password: String,
  pub new_password: String,
  #[serde(default)]
  pub logout_other_sessions: bool,
}

/// `PUT /v1/auth/password`
pub async fn change_password(
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
    &body.current_password,
    &body.new_password,
    body.logout_other_sessions,
    &auth.session.id,
    meta.ip.as_deref(),
  )
  .await?;

  let mut resp = (StatusCode::OK, Json(json!({ "ok": true }))).into_response();
  if let Some(token) = reminted.new_token {
    let cookie = cookie::set_cookie(&token, &state.cookie_domain, state.cookie_secure, SESSION_TTL.num_seconds());
    set_cookie_header(&mut resp, &cookie);
  }
  Ok(resp)
}

#[derive(Deserialize)]
pub struct ChangeEmailBody {
  pub new_email: String,
  pub current_password: String,
}

/// `PUT /v1/auth/email`
pub async fn change_email(
  State(state): State<AppState>,
  auth: AuthContext,
  headers: HeaderMap,
  Json(body): Json<ChangeEmailBody>,
) -> ApiResult<Json<Value>> {
  let meta = req_meta(&headers);
  let new_email =
    service::change_email(&state.pool, &auth.user, &body.new_email, &body.current_password, meta.ip.as_deref()).await?;
  // Email is now unverified again; the frontend re-fetches /me to reflect it.
  Ok(Json(json!({ "ok": true, "email": new_email, "email_verified": false })))
}

#[derive(Deserialize)]
pub struct PasswordResetRequestBody {
  pub email: String,
}

/// `POST /v1/auth/password-reset` — request a reset link. Always responds identically, whether or
/// not the address exists (auth.md §2 — no account enumeration).
pub async fn request_password_reset(
  State(state): State<AppState>,
  Json(body): Json<PasswordResetRequestBody>,
) -> ApiResult<Json<Value>> {
  if let Some(pending) = service::begin_password_reset(&state.pool, &body.email).await? {
    mailer::send_password_reset_email(&state, &pending.user_id, &pending.to_email, &pending.token).await;
  }
  Ok(Json(json!({
    "ok": true,
    "message": "If an account exists for that email, a reset link is on its way."
  })))
}

#[derive(Deserialize)]
pub struct PasswordResetConfirmBody {
  pub token: String,
  pub new_password: String,
  #[serde(default)]
  pub logout_other_sessions: bool,
}

/// `POST /v1/auth/password-reset/confirm` — consume a reset token and set the new password.
pub async fn confirm_password_reset(
  State(state): State<AppState>,
  headers: HeaderMap,
  Json(body): Json<PasswordResetConfirmBody>,
) -> ApiResult<Json<Value>> {
  let meta = req_meta(&headers);
  service::consume_password_reset(
    &state.pool,
    &body.token,
    &body.new_password,
    body.logout_other_sessions,
    meta.ip.as_deref(),
  )
  .await?;
  Ok(Json(json!({ "ok": true })))
}

/// `GET /v1/auth/sessions` — the devices list, with a `this_device` marker on the current session.
pub async fn list_sessions(State(state): State<AppState>, auth: AuthContext) -> ApiResult<Json<Value>> {
  let sessions = service::list_sessions(&state.pool, &auth.user.id).await?;
  let items: Vec<Value> = sessions
    .into_iter()
    .map(|s| {
      json!({
        "id": s.id,
        "ip": s.ip,
        "user_agent": s.user_agent,
        "device": s.device,
        "created_at": s.created_at,
        "expires_at": s.expires_at,
        "this_device": s.id == auth.session.id,
      })
    })
    .collect();
  Ok(Json(json!({ "sessions": items })))
}

/// `DELETE /v1/auth/sessions/{id}` — terminate one session.
pub async fn revoke_session(
  State(state): State<AppState>,
  auth: AuthContext,
  headers: HeaderMap,
  Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
  let session_id: ids::SessionId = id.parse().map_err(|_| ApiError::BadRequest("invalid session id".into()))?;
  let meta = req_meta(&headers);
  service::revoke_session(&state.pool, &auth.user, &session_id, meta.ip.as_deref()).await?;
  Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct RevokeSessionsQuery {
  #[serde(default)]
  pub others: bool,
}

/// `DELETE /v1/auth/sessions?others=true` — "log out all other sessions". Re-mints the current
/// cookie so this device stays signed in.
pub async fn revoke_sessions(
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
  let cookie = cookie::set_cookie(&token, &state.cookie_domain, state.cookie_secure, SESSION_TTL.num_seconds());
  let mut resp = (StatusCode::OK, Json(json!({ "ok": true }))).into_response();
  set_cookie_header(&mut resp, &cookie);
  Ok(resp)
}

// (avatar serving/upload/removal moved to `avatar_handlers` — accounts.md §2.)
