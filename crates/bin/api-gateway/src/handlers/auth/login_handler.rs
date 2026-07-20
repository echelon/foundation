//! `POST /v1/auth/login` — email/password login (auth.md §2).

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use serde::Deserialize;
use utoipa::ToSchema;

use crate::auth::service;
use crate::error::ApiResult;
use crate::handlers::support::{auth_success, req_meta, AuthSuccessResponse};
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginBody {
  pub email: String,
  pub password: String,
}

/// Log in. Failure is deliberately generic — it never reveals whether the email exists.
#[utoipa::path(
  post, path = "/v1/auth/login",
  tag = "auth",
  request_body = LoginBody,
  responses(
    (status = 200, description = "Logged in; session cookie set", body = AuthSuccessResponse),
    (status = 401, description = "Invalid email or password")
  )
)]
pub async fn login_handler(
  State(state): State<AppState>,
  headers: HeaderMap,
  Json(body): Json<LoginBody>,
) -> ApiResult<Response> {
  let outcome = service::login(&state.pool, &state.jwt, &body.email, &body.password, &req_meta(&headers)).await?;
  Ok(auth_success(&state, outcome, StatusCode::OK))
}
