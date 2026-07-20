//! `POST /v1/auth/email/verification/confirm` — consume a verification token. PUBLIC: the link
//! can be opened in any browser; the single-use token is the proof. Writes the org-visible
//! `email_verified` audit event.

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
pub struct EmailVerificationConfirmBody {
  pub token: String,
}

#[utoipa::path(
  post, path = "/v1/auth/email/verification/confirm",
  tag = "account",
  request_body = EmailVerificationConfirmBody,
  responses(
    (status = 200, description = "Email verified (idempotent)", body = OkResponse),
    (status = 400, description = "Invalid or expired token")
  )
)]
pub async fn confirm_email_verification_handler(
  State(state): State<AppState>,
  headers: HeaderMap,
  Json(body): Json<EmailVerificationConfirmBody>,
) -> ApiResult<Json<OkResponse>> {
  let meta = req_meta(&headers);
  service::confirm_email_verification(&state.pool, &body.token, meta.ip.as_deref()).await?;
  Ok(Json(OkResponse::ok()))
}
