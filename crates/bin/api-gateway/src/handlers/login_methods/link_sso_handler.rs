//! `POST /v1/auth/login_methods/{provider}` — connect Google/GitHub to the CURRENT user
//! (accounts.md §5 flows 1/3). The same proof material as login, but the resulting identity
//! attaches to this session's user instead of resolving to its own.

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use utoipa::ToSchema;

use super::types::{login_methods_response, LoginMethodsResponse};
use crate::auth::service;
use crate::auth::sso_verify::{verify_sso_identity, SsoProof};
use crate::error::ApiResult;
use crate::handlers::support::req_meta;
use crate::middleware::AuthContext;
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct LinkSsoBody {
  /// Google: the GIS credential (ID-token JWT).
  pub credential: Option<String>,
  /// GitHub: the authorization code.
  pub code: Option<String>,
  /// GitHub: the PKCE verifier for that code.
  pub code_verifier: Option<String>,
}

/// Provider email may differ from the account email (people have many); the connection is
/// never moved from another user — that's a 409.
#[utoipa::path(
  post, path = "/v1/auth/login_methods/{provider}",
  tag = "account",
  security(("session_cookie" = [])),
  params(("provider" = String, Path, description = "`google` or `github`")),
  request_body = LinkSsoBody,
  responses(
    (status = 200, description = "Connected (idempotent if already linked here); the updated list", body = LoginMethodsResponse),
    (status = 400, description = "Unsupported/unconfigured provider or missing proof"),
    (status = 401, description = "The credential/code failed verification"),
    (status = 409, description = "That provider account is already connected to a different user")
  )
)]
pub async fn link_sso_handler(
  State(state): State<AppState>,
  auth: AuthContext,
  headers: HeaderMap,
  Path(provider): Path<String>,
  Json(body): Json<LinkSsoBody>,
) -> ApiResult<Json<LoginMethodsResponse>> {
  let verified = verify_sso_identity(
    &state,
    &provider,
    SsoProof {
      credential: body.credential.as_deref(),
      code: body.code.as_deref(),
      code_verifier: body.code_verifier.as_deref(),
    },
  )
  .await?;

  let meta = req_meta(&headers);
  service::link_sso(
    &state.pool,
    &auth.user,
    service::SsoLoginArgs {
      provider: verified.provider,
      provider_user_id: &verified.sub,
      email: &verified.email,
      display_name: verified.display_name.as_deref(),
    },
    meta.ip.as_deref(),
  )
  .await?;
  Ok(Json(login_methods_response(&state, &auth.user).await?))
}
