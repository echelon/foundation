//! `GET /v1/invitations/{code_or_token}` — resolve for the join page (public).

use axum::extract::{Path, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

use ids::AccountId;

use crate::auth::service;
use crate::error::ApiResult;
use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct ResolveInvitationResponse {
  #[schema(value_type = String, example = "acct_01j9…")]
  pub account_id: AccountId,
  pub email: String,
  pub role: String,
  pub expires_at: DateTime<Utc>,
}

#[utoipa::path(
  get, path = "/v1/invitations/{code_or_token}",
  tag = "invitations",
  params(("code_or_token" = String, Path, description = "The short human code (`7F3K-QT9M`) or the magic-link token")),
  responses(
    (status = 200, description = "A live invitation", body = ResolveInvitationResponse),
    (status = 404, description = "Unknown, expired, revoked, or already-accepted invitation")
  )
)]
pub async fn resolve_invitation_handler(
  State(state): State<AppState>,
  Path(code_or_token): Path<String>,
) -> ApiResult<Json<ResolveInvitationResponse>> {
  let inv = service::resolve_invitation(&state.pool, &code_or_token).await?;
  Ok(Json(ResolveInvitationResponse {
    account_id: inv.account_id,
    email: inv.email,
    role: inv.role,
    expires_at: inv.expires_at,
  }))
}
