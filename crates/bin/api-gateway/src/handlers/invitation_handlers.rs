//! Invitation handlers (accounts.md §4, §6): create (owner/admin), resolve (join page), accept.

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use ids::{AccountId, InvitationId};

use crate::auth::service;
use crate::authz::require_account_role;
use crate::error::{ApiError, ApiResult};
use crate::middleware::AuthContext;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateInviteBody {
  pub email: String,
  /// `admin` | `member`.
  pub role: String,
}

/// `POST /v1/accounts/{account_id}/invitations` — owner/admin only.
pub async fn create_invitation(
  State(state): State<AppState>,
  auth: AuthContext,
  Path(account_id): Path<String>,
  Json(body): Json<CreateInviteBody>,
) -> ApiResult<Json<Value>> {
  let account_id: AccountId = account_id
    .parse()
    .map_err(|_| ApiError::BadRequest("invalid account id".into()))?;
  require_account_role(&state.pool, &auth.user.id, &account_id, &["owner", "admin"]).await?;

  let created = service::create_invitation(&state.pool, &account_id, &auth.user.id, &body.email, &body.role).await?;

  // The magic link carries the plaintext token (stored only as a hash).
  let join_url = format!("/join?token={}", created.token);
  Ok(Json(json!({
    "invitation_id": created.invitation.id,
    "code": created.code,
    "join_url": join_url,
    "email": created.invitation.email,
    "role": created.invitation.role,
    "expires_at": created.invitation.expires_at,
  })))
}

/// `GET /v1/invitations/{code-or-token}` — resolve for the join page (public).
pub async fn resolve_invitation(
  State(state): State<AppState>,
  Path(code_or_token): Path<String>,
) -> ApiResult<Json<Value>> {
  let inv = service::resolve_invitation(&state.pool, &code_or_token).await?;
  Ok(Json(json!({
    "account_id": inv.account_id,
    "email": inv.email,
    "role": inv.role,
    "expires_at": inv.expires_at,
  })))
}

/// `POST /v1/invitations/{id}/accept` — authenticated; creates the membership.
pub async fn accept_invitation(
  State(state): State<AppState>,
  auth: AuthContext,
  Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
  let invitation_id: InvitationId = id
    .parse()
    .map_err(|_| ApiError::BadRequest("invalid invitation id".into()))?;
  service::accept_invitation(&state.pool, &invitation_id, &auth.user.id).await?;
  Ok(Json(json!({ "ok": true, "invitation_id": invitation_id })))
}
