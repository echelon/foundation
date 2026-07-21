//! `GET /v1/auth/me` — the authenticated user, avatar included.

use axum::extract::State;
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use ids::SessionId;

use crate::avatar_links::avatar_info;
use crate::error::ApiResult;
use crate::handlers::api_types::UserBody;
use crate::middleware::AuthContext;
use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct MeResponse {
  pub user: UserBody,
  #[schema(value_type = String, example = "sess_01j9…")]
  pub session_id: SessionId,
  /// True when this is a staff impersonation session (auth.md §6).
  pub impersonation: bool,
  /// Connected SSO providers ("google", "github") — with `user.has_password`, the login-method
  /// factoids the settings profile shows.
  pub sso_providers: Vec<String>,
}

/// The caller's identity, including avatar state + CDN links (the gateway serves no bytes).
#[utoipa::path(
  get, path = "/v1/auth/me",
  tag = "auth",
  security(("session_cookie" = [])),
  responses(
    (status = 200, description = "The authenticated user", body = MeResponse),
    (status = 401, description = "Not logged in")
  )
)]
pub async fn me_handler(State(state): State<AppState>, auth: AuthContext) -> ApiResult<Json<MeResponse>> {
  let avatar = avatar_info(&state, &auth.user.id).await?;
  let mut user = UserBody::from_user(&auth.user);
  user.avatar = Some(avatar);
  let sso_providers = db_queries::sso::list_sso_for_user(&state.pool, &auth.user.id)
    .await?
    .into_iter()
    .map(|c| c.provider)
    .collect();
  Ok(Json(MeResponse {
    user,
    session_id: auth.session.id.clone(),
    impersonation: auth.is_impersonation(),
    sso_providers,
  }))
}
