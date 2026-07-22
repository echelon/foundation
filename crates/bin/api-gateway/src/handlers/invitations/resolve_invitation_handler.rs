//! `GET /v1/invitations/{code_or_token}` — resolve for the join page (public).

use axum::extract::{Path, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

use ids::{AccountId, InvitationId};

use crate::auth::service;
use crate::error::ApiResult;
use crate::state::AppState;

/// Everything the join page needs to branch (accounts.md §4): signed-in visitors confirm and
/// accept by `invitation_id`; signed-out visitors get login (`invitee_user_exists`) or a
/// signup form locked to `email`.
#[derive(Debug, Serialize, ToSchema)]
pub struct ResolveInvitationResponse {
  #[schema(value_type = String, example = "inv_01j9…")]
  pub invitation_id: InvitationId,
  #[schema(value_type = String, example = "acct_01j9…")]
  pub account_id: AccountId,
  /// The inviting organization's name (unnamed accounts report `null`).
  pub account_name: Option<String>,
  pub email: String,
  pub role: String,
  pub expires_at: DateTime<Utc>,
  /// A user already exists for the invited email → show login, not signup.
  pub invitee_user_exists: bool,
}

#[utoipa::path(
  get, path = "/v1/invitations/{code_or_token}",
  tag = "invitations",
  params(("code_or_token" = String, Path, description = "The short human code (`7F3K-QT9M`) or the magic-link token")),
  responses(
    (status = 200, description = "A live invitation", body = ResolveInvitationResponse),
    (status = 400, description = "Invitation expired, revoked, or already accepted"),
    (status = 404, description = "Unknown invitation")
  )
)]
pub async fn resolve_invitation_handler(
  State(state): State<AppState>,
  Path(code_or_token): Path<String>,
) -> ApiResult<Json<ResolveInvitationResponse>> {
  let resolved = service::resolve_invitation_for_join_page(&state.pool, &code_or_token).await?;
  Ok(Json(ResolveInvitationResponse {
    invitation_id: resolved.invitation.id.clone(),
    account_id: resolved.invitation.account_id,
    account_name: resolved.account_name,
    email: resolved.invitation.email,
    role: resolved.invitation.role.to_string(),
    expires_at: resolved.invitation.expires_at,
    invitee_user_exists: resolved.invitee_user_exists,
  }))
}
