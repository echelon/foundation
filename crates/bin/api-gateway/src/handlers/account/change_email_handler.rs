//! `PUT /v1/auth/email` — change email (accounts.md §2: five atomic steps; resets verification,
//! writes the org-visible `email_changed` audit event + the `email_changes` row).

use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::auth::service;
use crate::error::ApiResult;
use crate::handlers::support::req_meta;
use crate::middleware::AuthContext;
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct ChangeEmailBody {
  pub new_email: String,
  /// Fresh re-auth: the current password.
  pub current_password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ChangeEmailResponse {
  pub ok: bool,
  /// The new canonicalized address.
  pub email: String,
  /// Always false after a change — verification resets.
  pub email_verified: bool,
}

#[utoipa::path(
  put, path = "/v1/auth/email",
  tag = "account",
  security(("session_cookie" = [])),
  request_body = ChangeEmailBody,
  responses(
    (status = 200, description = "Email changed; verification reset", body = ChangeEmailResponse),
    (status = 401, description = "Current password incorrect"),
    (status = 409, description = "Email already in use")
  )
)]
pub async fn change_email_handler(
  State(state): State<AppState>,
  auth: AuthContext,
  headers: HeaderMap,
  Json(body): Json<ChangeEmailBody>,
) -> ApiResult<Json<ChangeEmailResponse>> {
  let meta = req_meta(&headers);
  let email =
    service::change_email(&state.pool, &auth.user, &body.new_email, &body.current_password, meta.ip.as_deref()).await?;
  Ok(Json(ChangeEmailResponse { ok: true, email, email_verified: false }))
}
