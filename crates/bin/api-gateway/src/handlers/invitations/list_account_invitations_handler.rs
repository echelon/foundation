//! `GET /v1/accounts/{account_id}/invitations` — the org's pending invitations.

use axum::extract::{Path, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

use db_queries::invitations;
use ids::{AccountId, InvitationId, UserId};

use crate::authz::require_account_role;
use crate::error::{ApiError, ApiResult};
use crate::middleware::AuthContext;
use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct PendingInvitationItem {
  #[schema(value_type = String, example = "inv_01j9…")]
  pub invitation_id: InvitationId,
  pub email: String,
  #[schema(example = "member")]
  pub role: String,
  /// The short human code — re-shareable from the console ("join with code").
  #[schema(example = "7F3K-QT9M")]
  pub code: String,
  /// Who issued it — resolve display info via `/v1/accounts/{id}/users`.
  #[schema(value_type = String, example = "user_01j9…")]
  pub invited_by_user_id: UserId,
  pub created_at: DateTime<Utc>,
  pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PendingInvitationsResponse {
  pub invitations: Vec<PendingInvitationItem>,
}

/// Owner/admin only — mirrors who can issue invitations (auth.md §4).
#[utoipa::path(
  get, path = "/v1/accounts/{account_id}/invitations",
  tag = "invitations",
  security(("session_cookie" = [])),
  params(("account_id" = String, Path, description = "Account id (`acct_…`)")),
  responses(
    (status = 200, description = "Open (unaccepted, unrevoked, unexpired) invitations, newest first", body = PendingInvitationsResponse),
    (status = 403, description = "Caller is a member but not owner/admin"),
    (status = 404, description = "Unknown account, or the caller is not a member")
  )
)]
pub async fn list_account_invitations_handler(
  State(state): State<AppState>,
  auth: AuthContext,
  Path(account_id): Path<String>,
) -> ApiResult<Json<PendingInvitationsResponse>> {
  let account_id: AccountId = account_id.parse().map_err(|_| ApiError::BadRequest("invalid account id".into()))?;
  require_account_role(&state.pool, &auth.user.id, &account_id, &["owner", "admin"]).await?;

  let pending = invitations::list_pending_invitations_for_account(&state.pool, &account_id).await?;
  let invitations = pending
    .into_iter()
    .map(|i| PendingInvitationItem {
      invitation_id: i.id,
      email: i.email,
      role: i.role,
      code: i.code,
      invited_by_user_id: i.invited_by_user_id,
      created_at: i.created_at,
      expires_at: i.expires_at,
    })
    .collect();
  Ok(Json(PendingInvitationsResponse { invitations }))
}
