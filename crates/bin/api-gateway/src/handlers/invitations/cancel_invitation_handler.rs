//! `DELETE /v1/accounts/{account_id}/invitations/{invitation_id}` — cancel a pending invite.

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;

use ids::{AccountId, InvitationId};

use crate::auth::service;
use crate::authz::require_account_role;
use crate::error::{ApiError, ApiResult};
use crate::handlers::api_types::OkResponse;
use crate::handlers::support::req_meta;
use crate::middleware::AuthContext;
use crate::state::AppState;

/// Owner/admin only. The magic link and code die immediately; the row is kept (revoked) and
/// the cancellation lands in the org audit log.
#[utoipa::path(
  delete, path = "/v1/accounts/{account_id}/invitations/{invitation_id}",
  tag = "invitations",
  security(("session_cookie" = [])),
  params(
    ("account_id" = String, Path, description = "Account id (`acct_…`)"),
    ("invitation_id" = String, Path, description = "Invitation id (`inv_…`)")
  ),
  responses(
    (status = 200, description = "Invitation cancelled", body = OkResponse),
    (status = 400, description = "Invalid id"),
    (status = 403, description = "Caller is a member but not owner/admin"),
    (status = 404, description = "Unknown/foreign invitation, already closed, or caller is not a member")
  )
)]
pub async fn cancel_invitation_handler(
  State(state): State<AppState>,
  auth: AuthContext,
  headers: HeaderMap,
  Path((account_id, invitation_id)): Path<(String, String)>,
) -> ApiResult<Json<OkResponse>> {
  let account_id: AccountId = account_id.parse().map_err(|_| ApiError::BadRequest("invalid account id".into()))?;
  let invitation_id: InvitationId =
    invitation_id.parse().map_err(|_| ApiError::BadRequest("invalid invitation id".into()))?;
  require_account_role(&state.pool, &auth.user.id, &account_id, &["owner", "admin"]).await?;

  let meta = req_meta(&headers);
  service::cancel_invitation(&state.pool, &account_id, &auth.user.id, &invitation_id, meta.ip.as_deref()).await?;
  Ok(Json(OkResponse::ok()))
}
