//! `POST /v1/auth/password-reset/confirm` — consume a reset token and set the new password.

use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use utoipa::ToSchema;

use crate::auth::service;
use crate::error::ApiResult;
use crate::handlers::api_types::OkResponse;
use crate::handlers::support::req_meta;
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct PasswordResetConfirmBody {
  pub token: String,
  pub new_password: String,
  /// End every existing session (the reset flow has no "current" session to keep).
  #[serde(default)]
  pub logout_other_sessions: bool,
}

#[utoipa::path(
  post, path = "/v1/auth/password-reset/confirm",
  tag = "account",
  request_body = PasswordResetConfirmBody,
  responses(
    (status = 200, description = "Password set", body = OkResponse),
    (status = 400, description = "Invalid/expired token or password too short")
  )
)]
pub async fn confirm_password_reset_handler(
  State(state): State<AppState>,
  headers: HeaderMap,
  Json(body): Json<PasswordResetConfirmBody>,
) -> ApiResult<Json<OkResponse>> {
  let meta = req_meta(&headers);
  service::consume_password_reset(
    &state.pool,
    &body.token,
    &body.new_password,
    body.logout_other_sessions,
    meta.ip.as_deref(),
  )
  .await?;
  Ok(Json(OkResponse::ok()))
}
