//! `POST /v1/auth/oauth/{provider}/callback` — SSO login/signup (accounts.md §3B/C).
//!
//! Google uses the modern Google Identity Services flow: the frontend's GIS button hands us
//! a **credential** (an ID-token JWT), verified server-side in [`crate::auth::google`]
//! (signature via Google's JWKS, `aud` = our client id, `iss` = accounts.google.com, `exp`).
//! The client-supplied token proves the identity — nothing else from the client is trusted.

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use serde::Deserialize;
use utoipa::ToSchema;

use crate::auth::service;
use crate::error::{ApiError, ApiResult};
use crate::handlers::support::{auth_success, req_meta, AuthSuccessResponse};
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct OAuthCallbackBody {
  /// Google: the GIS credential (ID-token JWT) from `google.accounts.id`.
  pub credential: Option<String>,
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
  // Resolve a VERIFIED identity per provider — nothing about identity is client-trusted.
  struct Verified {
    provider: &'static str,
    sub: String,
    email: String,
    display_name: Option<String>,
  }
  let verified: Verified = match provider.as_str() {
    "google" => {
      let google = state
        .google
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("Google sign-in is not configured on this server".into()))?;
      let credential =
        body.credential.as_deref().ok_or_else(|| ApiError::BadRequest("`credential` is required for google".into()))?;
      let id = google.verify(credential).await.map_err(|e| {
        tracing::warn!(error = %e, "google credential rejected");
        ApiError::Unauthorized("Google credential could not be verified".into())
      })?;
      // We mark SSO emails verified (accounts.md §3B) — so only accept ones Google vouches for.
      if !id.email_verified {
        return Err(ApiError::Unauthorized("Google reports this email as unverified".into()));
      }
      Verified { provider: "google", sub: id.sub, email: id.email, display_name: id.display_name }
    }
    "github" => {
      let github = state
        .github
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("GitHub sign-in is not configured on this server".into()))?;
      let code = body.code.as_deref().ok_or_else(|| ApiError::BadRequest("`code` is required for github".into()))?;
      // resolve_code enforces a VERIFIED email via /user/emails (accounts.md §3C).
      let id = github.resolve_code(code, body.code_verifier.as_deref()).await.map_err(|e| {
        tracing::warn!(error = %e, "github code rejected");
        ApiError::Unauthorized("GitHub sign-in could not be verified".into())
      })?;
      Verified { provider: "github", sub: id.sub, email: id.email, display_name: id.display_name }
    }
    other => return Err(ApiError::BadRequest(format!("unsupported provider: {other}"))),
  };

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
