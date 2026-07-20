//! The response every avatar mutation answers with: the resulting avatar state.

use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use crate::avatar_links::{avatar_info, AvatarInfo};
use crate::error::ApiResult;
use crate::middleware::AuthContext;
use crate::state::AppState;

/// Acknowledgement + the avatar as it now stands (source + CDN links).
#[derive(Debug, Serialize, ToSchema)]
pub struct AvatarResponse {
  pub ok: bool,
  pub avatar: AvatarInfo,
}

/// Every avatar mutation answers with the resulting avatar (source + CDN links).
pub async fn avatar_response(state: &AppState, auth: &AuthContext) -> ApiResult<Json<AvatarResponse>> {
  let avatar = avatar_info(state, &auth.user.id).await?;
  Ok(Json(AvatarResponse { ok: true, avatar }))
}
