//! `POST /v1/auth/avatar/gravatar/refresh` — re-request the gravatar.

use axum::extract::State;
use axum::Json;

use db_queries::user_avatars;

use super::support::{avatar_response, AvatarResponse};
use crate::error::ApiResult;
use crate::middleware::AuthContext;
use crate::state::AppState;

/// Resets the poll marker so the gravatar-job fetches it again on its next pass (typically
/// within seconds). The current mirror keeps displaying until the re-poll replaces or clears
/// it; `avatar.gravatar_checked` stays false in the meantime so clients can show "checking…".
#[utoipa::path(
  post, path = "/v1/auth/avatar/gravatar/refresh",
  tag = "avatar",
  security(("session_cookie" = [])),
  responses((status = 200, description = "Re-poll requested; the current avatar", body = AvatarResponse))
)]
pub async fn refresh_my_gravatar_handler(
  State(state): State<AppState>,
  auth: AuthContext,
) -> ApiResult<Json<AvatarResponse>> {
  user_avatars::request_gravatar_repoll(&state.pool, &auth.user.id).await?;
  avatar_response(&state, &auth).await
}
