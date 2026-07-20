//! `PUT /v1/auth/profile` — update the display name (org-visible `display_name_changed` audit).

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
pub struct UpdateProfileBody {
  /// New display name; empty/absent clears it.
  pub display_name: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UpdateProfileResponse {
  pub ok: bool,
  pub display_name: Option<String>,
}

#[utoipa::path(
  put, path = "/v1/auth/profile",
  tag = "account",
  security(("session_cookie" = [])),
  request_body = UpdateProfileBody,
  responses(
    (status = 200, description = "Name updated (audit-logged, org-visible)", body = UpdateProfileResponse),
    (status = 400, description = "Name too long")
  )
)]
pub async fn update_profile_handler(
  State(state): State<AppState>,
  auth: AuthContext,
  headers: HeaderMap,
  Json(body): Json<UpdateProfileBody>,
) -> ApiResult<Json<UpdateProfileResponse>> {
  let meta = req_meta(&headers);
  let display_name =
    service::change_display_name(&state.pool, &auth.user, body.display_name.as_deref(), meta.ip.as_deref()).await?;
  Ok(Json(UpdateProfileResponse { ok: true, display_name }))
}
