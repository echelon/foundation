//! `GET /v1/auth/sessions` — the devices list (auth.md §3).

use axum::extract::State;
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use chrono::{DateTime, Utc};
use ids::SessionId;

use crate::auth::service;
use crate::error::ApiResult;
use crate::middleware::AuthContext;
use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct SessionItem {
  #[schema(value_type = String, example = "sess_01j9…")]
  pub id: SessionId,
  pub ip: Option<String>,
  pub user_agent: Option<String>,
  pub device: Option<String>,
  /// How the session came to exist: `signup`, `invited_signup`, `login`, `sso_login`, ….
  #[schema(value_type = String, example = "login")]
  pub created_via: enums::SessionCreationType,
  pub created_at: DateTime<Utc>,
  pub expires_at: DateTime<Utc>,
  /// Marks the session making this request.
  pub this_device: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SessionsResponse {
  pub sessions: Vec<SessionItem>,
}

#[utoipa::path(
  get, path = "/v1/auth/sessions",
  tag = "sessions",
  security(("session_cookie" = [])),
  responses((status = 200, description = "Live sessions, newest first", body = SessionsResponse))
)]
pub async fn list_sessions_handler(State(state): State<AppState>, auth: AuthContext) -> ApiResult<Json<SessionsResponse>> {
  let sessions = service::list_sessions(&state.pool, &auth.user.id).await?;
  let sessions = sessions
    .into_iter()
    .map(|s| SessionItem {
      this_device: s.id == auth.session.id,
      id: s.id,
      ip: s.ip,
      user_agent: s.user_agent,
      device: s.device,
      created_via: s.created_via,
      created_at: s.created_at,
      expires_at: s.expires_at,
    })
    .collect();
  Ok(Json(SessionsResponse { sessions }))
}
