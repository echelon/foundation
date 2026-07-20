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
  /// An invitation code or magic-link token to redeem during signup (accounts.md §4).
  /// The email must match the invite's email; the new user joins the inviting organization
  /// instead of getting a personal one.
  #[schema(example = "7F3K-QT9M")]
  pub invitation: Option<String>,
}

/// Create a user and log in (sets the session cookie). Without `invitation` the user gets
/// their own first organization; with it they instead join the inviting organization in the
/// invite's role (invites never create accounts — only memberships).
#[utoipa::path(
  post, path = "/v1/auth/signup",
  tag = "auth",
  request_body = SignupBody,
  responses(
    (status = 201, description = "User created; session cookie set", body = AuthSuccessResponse),
    (status = 400, description = "Password too short, dead invitation, or invitation email mismatch"),
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
    service::SignupArgs {
      email: &body.email,
      password: &body.password,
      display_name: body.display_name.as_deref(),
      invitation: body.invitation.as_deref(),
    },
    &req_meta(&headers),
  )
  .await?;
  Ok(auth_success(&state, outcome, StatusCode::CREATED))
}
