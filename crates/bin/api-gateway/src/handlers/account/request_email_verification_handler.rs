//! `POST /v1/auth/email/verification` — send (or re-send) the verification email.

use axum::extract::State;
use axum::Json;

use crate::auth::service;
use crate::error::ApiResult;
use crate::handlers::api_types::MessageResponse;
use crate::mailer;
use crate::middleware::AuthContext;
use crate::state::AppState;

#[utoipa::path(
  post, path = "/v1/auth/email/verification",
  tag = "account",
  security(("session_cookie" = [])),
  responses(
    (status = 200, description = "Verification email sent", body = MessageResponse),
    (status = 400, description = "Already verified")
  )
)]
pub async fn request_email_verification_handler(
  State(state): State<AppState>,
  auth: AuthContext,
) -> ApiResult<Json<MessageResponse>> {
  let pending = service::request_email_verification(&state.pool, &auth.user).await?;
  mailer::send_verification_email(&state, &auth.user.id, &pending.to_email, &pending.token).await;
  Ok(Json(MessageResponse { ok: true, message: "Verification email sent — check your inbox.".into() }))
}
