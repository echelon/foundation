//! `DELETE /v1/auth/avatar` — remove the custom avatar (file rows kept, pointer cleared).

use axum::extract::State;
use axum::Json;

use db_queries::user_avatars;

use super::support::{avatar_response, AvatarResponse};
use crate::error::ApiResult;
use crate::middleware::AuthContext;
use crate::state::AppState;

/// A `custom` preference falls back to the gravatar if mirrored, else the monogram.
#[utoipa::path(
  delete, path = "/v1/auth/avatar",
  tag = "avatar",
  security(("session_cookie" = [])),
  responses((status = 200, description = "Removed; the resulting avatar", body = AvatarResponse))
)]
pub async fn remove_my_avatar_handler(
  State(state): State<AppState>,
  auth: AuthContext,
) -> ApiResult<Json<AvatarResponse>> {
  user_avatars::remove_custom_avatar(&state.pool, &auth.user.id).await?;
  avatar_response(&state, &auth).await
}
