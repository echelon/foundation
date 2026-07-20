//! `POST /v1/auth/password-reset` — request a reset link. Always answers identically, whether
//! or not the address exists (no account enumeration — auth.md §2).

use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use utoipa::ToSchema;

use crate::auth::service;
use crate::error::ApiResult;
use crate::handlers::api_types::MessageResponse;
use crate::mailer;
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct PasswordResetRequestBody {
  pub email: String,
}

#[utoipa::path(
  post, path = "/v1/auth/password-reset",
  tag = "account",
  request_body = PasswordResetRequestBody,
  responses((status = 200, description = "Identical whether or not the account exists", body = MessageResponse))
)]
pub async fn request_password_reset_handler(
  State(state): State<AppState>,
  Json(body): Json<PasswordResetRequestBody>,
) -> ApiResult<Json<MessageResponse>> {
  if let Some(pending) = service::begin_password_reset(&state.pool, &body.email).await? {
    mailer::send_password_reset_email(&state, &pending.user_id, &pending.to_email, &pending.token).await;
  }
  Ok(Json(MessageResponse { ok: true, message: "If an account exists for that email, a reset link is on its way.".into() }))
}
