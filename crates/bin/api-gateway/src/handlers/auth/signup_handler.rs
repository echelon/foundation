//! `POST /v1/auth/signup` — email/password signup (accounts.md §3A).

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
pub struct SignupBody {
  #[schema(example = "you@company.com")]
  pub email: String,
  #[schema(example = "correct horse battery staple")]
  pub password: String,
  pub display_name: Option<String>,
}

/// Create a user + their first organization and log in (sets the session cookie).
#[utoipa::path(
  post, path = "/v1/auth/signup",
  tag = "auth",
  request_body = SignupBody,
  responses(
    (status = 201, description = "Account created; session cookie set", body = AuthSuccessResponse),
    (status = 400, description = "Password too short"),
    (status = 409, description = "Email already registered (with a different password)")
  )
)]
pub async fn signup_handler(
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
