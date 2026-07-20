//! Shared handler plumbing: request metadata extraction and session-cookie response helpers.

use axum::http::header::{SET_COOKIE, USER_AGENT};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::auth::cookie;
use crate::auth::service::{AuthOutcome, ReqMeta};
use crate::auth::token::SESSION_TTL;
use crate::handlers::api_types::UserBody;
use crate::state::AppState;

use ids::SessionId;
use serde::Serialize;
use utoipa::ToSchema;

/// Extract client ip (x-forwarded-for) + user agent for session rows / audit.
pub fn req_meta(headers: &HeaderMap) -> ReqMeta {
  let ip = headers
    .get("x-forwarded-for")
    .and_then(|v| v.to_str().ok())
    .and_then(|s| s.split(',').next())
    .map(|s| s.trim().to_string())
    .filter(|s| !s.is_empty());
  let user_agent = headers.get(USER_AGENT).and_then(|v| v.to_str().ok()).map(str::to_string);
  ReqMeta { ip, user_agent }
}

pub fn set_cookie_header(resp: &mut Response, cookie: &str) {
  if let Ok(value) = HeaderValue::from_str(cookie) {
    resp.headers_mut().append(SET_COOKIE, value);
  }
}

/// The logged-in response body: the user + their new session id (the token itself travels only
/// in the HttpOnly cookie).
#[derive(Debug, Serialize, ToSchema)]
pub struct AuthSuccessResponse {
  pub user: UserBody,
  #[schema(value_type = String, example = "sess_01j9…")]
  pub session_id: SessionId,
}

/// Build the logged-in response: set the session cookie + return the typed body.
pub fn auth_success(state: &AppState, outcome: AuthOutcome, status: StatusCode) -> Response {
  let cookie =
    cookie::set_cookie(&outcome.token, &state.cookie_domain, state.cookie_secure, SESSION_TTL.num_seconds());
  let body = AuthSuccessResponse { user: UserBody::from_user(&outcome.user), session_id: outcome.session.id.clone() };
  let mut resp = (status, Json(body)).into_response();
  set_cookie_header(&mut resp, &cookie);
  resp
}

/// Re-set the session cookie on a typed response (used after `session_version` bumps).
pub fn with_reminted_cookie<T: Serialize>(state: &AppState, token: &str, body: T) -> Response {
  let cookie = cookie::set_cookie(token, &state.cookie_domain, state.cookie_secure, SESSION_TTL.num_seconds());
  let mut resp = (StatusCode::OK, Json(body)).into_response();
  set_cookie_header(&mut resp, &cookie);
  resp
}
