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
  /// The GIS credential: the ID-token JWT from `google.accounts.id` (verified server-side).
  pub credential: String,
}

#[utoipa::path(
  post, path = "/v1/auth/oauth/{provider}/callback",
  tag = "auth",
  params(("provider" = String, Path, description = "`google` (github: not yet available)")),
  request_body = OAuthCallbackBody,
  responses(
    (status = 200, description = "Logged in (created on first visit); session cookie set", body = AuthSuccessResponse),
    (status = 400, description = "Unsupported provider, or Google SSO not configured on this server"),
    (status = 401, description = "The credential failed verification"),
    (status = 409, description = "Email exists without this SSO link — log in to link")
  )
)]
pub async fn oauth_callback_handler(
  State(state): State<AppState>,
  Path(provider): Path<String>,
  headers: HeaderMap,
  Json(body): Json<OAuthCallbackBody>,
) -> ApiResult<Response> {
  if provider != "google" {
    return Err(ApiError::BadRequest(format!("unsupported provider: {provider}")));
  }
  let google = state
    .google
    .as_ref()
    .ok_or_else(|| ApiError::BadRequest("Google sign-in is not configured on this server".into()))?;

  let identity = google.verify(&body.credential).await.map_err(|e| {
    tracing::warn!(error = %e, "google credential rejected");
    ApiError::Unauthorized("Google credential could not be verified".into())
  })?;
  // We mark SSO emails verified (accounts.md §3B) — so only accept ones Google vouches for.
  if !identity.email_verified {
    return Err(ApiError::Unauthorized("Google reports this email as unverified".into()));
  }

  let outcome = service::oauth_login(
    &state.pool,
    &state.jwt,
    service::SsoLoginArgs {
      provider: "google",
      provider_user_id: &identity.sub,
      email: &identity.email,
      display_name: identity.display_name.as_deref(),
    },
    &req_meta(&headers),
  )
  .await?;
  Ok(auth_success(&state, outcome, StatusCode::OK))
}
