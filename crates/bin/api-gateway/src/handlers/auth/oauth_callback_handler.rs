//! `POST /v1/auth/oauth/{provider}/callback` — SSO login/signup (accounts.md §3B/C).
//!
//! Simplified callback: in the full flow the provider `code` is exchanged for the identity;
//! here the already-resolved identity is accepted (the OAuth token exchange is the stubbed
//! part).

use axum::extract::{Path, State};
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
pub struct OAuthCallbackBody {
  /// Stable provider subject (`sub` / numeric id) — the identity anchor, never the email.
  pub provider_user_id: String,
  pub email: String,
  pub display_name: Option<String>,
}

#[utoipa::path(
  post, path = "/v1/auth/oauth/{provider}/callback",
  tag = "auth",
  params(("provider" = String, Path, description = "`google` or `github`")),
  request_body = OAuthCallbackBody,
  responses(
    (status = 200, description = "Logged in (created on first visit); session cookie set", body = AuthSuccessResponse),
    (status = 400, description = "Unsupported provider"),
    (status = 409, description = "Email exists without this SSO link — log in to link")
  )
)]
pub async fn oauth_callback_handler(
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
