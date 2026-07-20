//! Auth handlers: signup, login, logout, current-user, SSO callback (accounts.md §6).

use axum::extract::{Path, State};
use axum::http::header::{SET_COOKIE, USER_AGENT};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::cookie;
use crate::auth::service::{self, AuthOutcome, ReqMeta};
use crate::auth::token::SESSION_TTL;
use crate::error::ApiResult;
use crate::middleware::AuthContext;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct SignupBody {
  pub email: String,
  pub password: String,
  pub display_name: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginBody {
  pub email: String,
  pub password: String,
}

/// Simplified SSO callback: in a real flow the provider `code` is exchanged for the identity;
/// here we accept the already-resolved identity (the OAuth token exchange is the stubbed part).
#[derive(Deserialize)]
pub struct OAuthCallbackBody {
  /// Stable provider id (`sub` / numeric github id).
  pub provider_user_id: String,
  pub email: String,
  pub display_name: Option<String>,
}

/// `POST /v1/auth/signup`
pub async fn signup(
  State(state): State<AppState>,
  headers: HeaderMap,
  Json(body): Json<SignupBody>,
) -> ApiResult<Response> {
  let outcome = service::signup(
    &state.pool,
    &state.jwt,
    &body.email,
    &body.password,
    body.display_name.as_deref(),
    &req_meta(&headers),
  )
  .await?;
  Ok(auth_success(&state, outcome, StatusCode::CREATED))
}

/// `POST /v1/auth/login`
pub async fn login(
  State(state): State<AppState>,
  headers: HeaderMap,
  Json(body): Json<LoginBody>,
) -> ApiResult<Response> {
  let outcome = service::login(&state.pool, &state.jwt, &body.email, &body.password, &req_meta(&headers)).await?;
  Ok(auth_success(&state, outcome, StatusCode::OK))
}

/// `POST /v1/auth/logout` — revoke the current session and clear the cookie.
pub async fn logout(State(state): State<AppState>, auth: AuthContext) -> ApiResult<Response> {
  service::logout(&state.pool, &auth.session.id).await?;
  let clear = cookie::clear_cookie(&state.cookie_domain, state.cookie_secure);
  let mut resp = (StatusCode::OK, Json(json!({ "ok": true }))).into_response();
  set_cookie_header(&mut resp, &clear);
  Ok(resp)
}

/// `GET /v1/auth/me` — the authenticated user (proves the middleware works end-to-end).
/// Includes the avatar display `source` (accounts.md §2 precedence) so the frontend knows
/// whether to load `/v1/auth/avatar` or fall back to the letter monogram.
pub async fn me(State(state): State<AppState>, auth: AuthContext) -> crate::error::ApiResult<Json<Value>> {
  let avatar = db_queries::user_avatars::find_user_avatar_by_user(&state.pool, &auth.user.id).await?;
  let mut user = user_json(&auth.user);
  user["avatar_source"] = avatar.map(|a| a.source).unwrap_or_else(|| "letter".into()).into();
  Ok(Json(json!({
    "user": user,
    "session_id": auth.session.id,
    "impersonation": auth.is_impersonation(),
  })))
}

/// `POST /v1/auth/oauth/{provider}/callback`
pub async fn oauth_callback(
  State(state): State<AppState>,
  Path(provider): Path<String>,
  headers: HeaderMap,
  Json(body): Json<OAuthCallbackBody>,
) -> ApiResult<Response> {
  let outcome = service::oauth_login(
    &state.pool,
    &state.jwt,
    &provider,
    &body.provider_user_id,
    &body.email,
    body.display_name.as_deref(),
    &req_meta(&headers),
  )
  .await?;
  Ok(auth_success(&state, outcome, StatusCode::OK))
}

/* ── helpers ── */

fn user_json(u: &db_queries::User) -> Value {
  json!({
    "id": u.id,
    "email": u.email,
    "display_name": u.display_name,
    "is_staff": u.is_staff,
    "email_verified": u.email_verified_at.is_some(),
  })
}

/// Build a success response: set the session cookie + return the user.
fn auth_success(state: &AppState, outcome: AuthOutcome, status: StatusCode) -> Response {
  let cookie = cookie::set_cookie(&outcome.token, &state.cookie_domain, state.cookie_secure, SESSION_TTL.num_seconds());
  let mut resp = (status, Json(json!({ "user": user_json(&outcome.user), "session_id": outcome.session.id }))).into_response();
  set_cookie_header(&mut resp, &cookie);
  resp
}

pub(crate) fn set_cookie_header(resp: &mut Response, cookie: &str) {
  if let Ok(value) = HeaderValue::from_str(cookie) {
    resp.headers_mut().append(SET_COOKIE, value);
  }
}

pub(crate) fn req_meta(headers: &HeaderMap) -> ReqMeta {
  let ip = headers
    .get("x-forwarded-for")
    .and_then(|v| v.to_str().ok())
    .and_then(|s| s.split(',').next())
    .map(|s| s.trim().to_string())
    .filter(|s| !s.is_empty());
  let user_agent = headers.get(USER_AGENT).and_then(|v| v.to_str().ok()).map(str::to_string);
  ReqMeta { ip, user_agent }
}
