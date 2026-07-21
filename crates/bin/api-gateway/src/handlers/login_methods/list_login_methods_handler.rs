//! `GET /v1/auth/login_methods` — every way this user can sign in.

use axum::extract::State;
use axum::Json;

use super::types::{login_methods_response, LoginMethodsResponse};
use crate::error::ApiResult;
use crate::middleware::AuthContext;
use crate::state::AppState;

#[utoipa::path(
  get, path = "/v1/auth/login_methods",
  tag = "account",
  security(("session_cookie" = [])),
  responses((status = 200, description = "The password entry + every SSO connection", body = LoginMethodsResponse))
)]
pub async fn list_login_methods_handler(
  State(state): State<AppState>,
  auth: AuthContext,
) -> ApiResult<Json<LoginMethodsResponse>> {
  Ok(Json(login_methods_response(&state, &auth.user).await?))
}
