//! `POST /v1/auth/oauth/{provider}/callback` — SSO login/signup (accounts.md §3B/C).
//!
//! Google: the GIS credential (ID-token JWT), verified in [`crate::auth::google`].
//! GitHub: the web-application-flow code (+ PKCE verifier), exchanged server-side in
//! [`crate::auth::github`]. Either way the proof is verified in
//! [`crate::auth::sso_verify`] — nothing identity-shaped is client-trusted.

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use serde::Deserialize;
use utoipa::ToSchema;

use crate::auth::service;
use crate::auth::sso_verify::{verify_sso_identity, SsoProof};
use crate::error::ApiResult;
use crate::handlers::support::{auth_success, req_meta, AuthSuccessResponse};
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct OAuthCallbackBody {
  /// Google: the GIS credential (ID-token JWT) from `google.accounts.id`.
  pub credential: Option<String>,
  /// Google: a token-client access token (custom-button flow) — alternative to `credential`.
  pub access_token: Option<String>,
  /// GitHub: the authorization code from the web application flow.
  pub code: Option<String>,
  /// GitHub: the PKCE verifier that produced the authorize request's `code_challenge`.
  pub code_verifier: Option<String>,
}

#[utoipa::path(
  post, path = "/v1/auth/oauth/{provider}/callback",
  tag = "auth",
  params(("provider" = String, Path, description = "`google` (send `credential`) or `github` (send `code` + `code_verifier`)")),
  request_body = OAuthCallbackBody,
  responses(
    (status = 200, description = "Logged in (created on first visit); session cookie set", body = AuthSuccessResponse),
    (status = 400, description = "Unsupported provider, missing field, or that provider is not configured on this server"),
    (status = 401, description = "The credential/code failed verification"),
    (status = 409, description = "Email exists without this SSO link — log in to link")
  )
)]
pub async fn oauth_callback_handler(
  State(state): State<AppState>,
  Path(provider): Path<String>,
  headers: HeaderMap,
  Json(body): Json<OAuthCallbackBody>,
) -> ApiResult<Response> {
  let verified = verify_sso_identity(
    &state,
    &provider,
    SsoProof {
      credential: body.credential.as_deref(),
      access_token: body.access_token.as_deref(),
      code: body.code.as_deref(),
      code_verifier: body.code_verifier.as_deref(),
    },
  )
  .await?;

  let outcome = service::oauth_login(
    &state.pool,
    &state.jwt,
    service::SsoLoginArgs {
      provider: verified.provider,
      provider_user_id: &verified.sub,
      email: &verified.email,
      display_name: verified.display_name.as_deref(),
    },
    &req_meta(&headers),
  )
  .await?;
  Ok(auth_success(&state, outcome, StatusCode::OK))
}
