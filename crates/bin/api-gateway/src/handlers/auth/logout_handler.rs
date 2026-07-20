//! `POST /v1/auth/logout` — revoke the current session + clear the cookie (auth.md §3).

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::auth::cookie;
use crate::auth::service;
use crate::error::ApiResult;
use crate::handlers::api_types::OkResponse;
use crate::handlers::support::set_cookie_header;
use crate::middleware::AuthContext;
use crate::state::AppState;

#[utoipa::path(
  post, path = "/v1/auth/logout",
  tag = "auth",
  security(("session_cookie" = [])),
  responses(
    (status = 200, description = "Session revoked; cookie cleared", body = OkResponse),
    (status = 401, description = "Not logged in")
  )
)]
pub async fn logout_handler(State(state): State<AppState>, auth: AuthContext) -> ApiResult<Response> {
  service::logout(&state.pool, &auth.session.id).await?;
  let clear = cookie::clear_cookie(&state.cookie_domain, state.cookie_secure);
  let mut resp = (StatusCode::OK, Json(OkResponse::ok())).into_response();
  set_cookie_header(&mut resp, &clear);
  Ok(resp)
}
