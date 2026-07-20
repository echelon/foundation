//! `DELETE /v1/auth/avatar/gravatar` — remove the mirrored gravatar.

use axum::extract::State;
use axum::Json;

use db_queries::user_avatars;

use super::support::{avatar_response, AvatarResponse};
use crate::error::ApiResult;
use crate::middleware::AuthContext;
use crate::state::AppState;

/// It won't re-mirror on its own (the probe stays stamped); an explicit refresh brings it back.
/// A `gravatar` preference falls back to the custom upload if one exists, else the monogram.
#[utoipa::path(
  delete, path = "/v1/auth/avatar/gravatar",
  tag = "avatar",
  security(("session_cookie" = [])),
  responses((status = 200, description = "Removed; the resulting avatar", body = AvatarResponse))
)]
pub async fn remove_my_gravatar_handler(
  State(state): State<AppState>,
  auth: AuthContext,
) -> ApiResult<Json<AvatarResponse>> {
  user_avatars::remove_gravatar(&state.pool, &auth.user.id).await?;
  avatar_response(&state, &auth).await
}
