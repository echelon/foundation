//! `PUT /v1/auth/avatar/monogram` — pick the initials-monogram gradient.

use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use utoipa::ToSchema;

use db_queries::user_avatars;
use enums::MonogramGradient;

use super::support::{avatar_response, AvatarResponse};
use crate::error::{ApiError, ApiResult};
use crate::middleware::AuthContext;
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct MonogramGradientBody {
  /// Typed via `enums::MonogramGradient` — unknown slugs are rejected at deserialization.
  #[schema(value_type = String, example = "ocean")]
  pub gradient: MonogramGradient,
}

/// Stored per user and reported in every avatar payload, so the monogram renders in the
/// chosen colors everywhere (roster, audit log, header — not just for the owner).
#[utoipa::path(
  put, path = "/v1/auth/avatar/monogram",
  tag = "avatar",
  security(("session_cookie" = [])),
  request_body = MonogramGradientBody,
  responses(
    (status = 200, description = "Gradient stored; the resulting avatar", body = AvatarResponse),
    (status = 400, description = "Unknown gradient")
  )
)]
pub async fn set_monogram_gradient_handler(
  State(state): State<AppState>,
  auth: AuthContext,
  Json(body): Json<MonogramGradientBody>,
) -> ApiResult<Json<AvatarResponse>> {
  let updated = user_avatars::set_monogram_gradient(&state.pool, &auth.user.id, body.gradient).await?;
  if !updated {
    // Every signup provisions an avatar row; a miss means something is genuinely wrong.
    return Err(ApiError::Internal("no avatar row for this user".into()));
  }
  avatar_response(&state, &auth).await
}
