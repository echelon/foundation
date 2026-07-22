//! `DELETE /v1/accounts/{account_id}/members/{user_id}` — remove a member.

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;

use ids::{AccountId, UserId};

use crate::auth::service;
use crate::authz::require_account_role;
use crate::error::{ApiError, ApiResult};
use crate::handlers::api_types::OkResponse;
use crate::handlers::support::req_meta;
use crate::middleware::AuthContext;
use crate::state::AppState;

/// Owner/admin only; the service enforces the finer rules (no self-removal, owner immovable,
/// admins can't remove admins) and writes the `member_removed` audit rows.
#[utoipa::path(
  delete, path = "/v1/accounts/{account_id}/members/{user_id}",
  tag = "org",
  security(("session_cookie" = [])),
  params(
    ("account_id" = String, Path, description = "Account id (`acct_…`)"),
    ("user_id" = String, Path, description = "The member's user id (`user_…`)")
  ),
  responses(
    (status = 200, description = "Member removed", body = OkResponse),
    (status = 400, description = "Invalid id, or a removal the rules forbid"),
    (status = 404, description = "Not a member, or not admin/owner")
  )
)]
pub async fn remove_account_member_handler(
  State(state): State<AppState>,
  auth: AuthContext,
  headers: HeaderMap,
  Path((account_id, user_id)): Path<(String, String)>,
) -> ApiResult<Json<OkResponse>> {
  let account_id: AccountId = account_id.parse().map_err(|_| ApiError::BadRequest("invalid account id".into()))?;
  let target_user_id: UserId = user_id.parse().map_err(|_| ApiError::BadRequest("invalid user id".into()))?;

  let my_membership = require_account_role(&state.pool, &auth.user.id, &account_id, enums::MemberRole::ADMINS).await?;
  let meta = req_meta(&headers);
  service::remove_member(
    &state.pool,
    &auth.user,
    my_membership.role,
    &account_id,
    &target_user_id,
    meta.ip.as_deref(),
  )
  .await?;
  Ok(Json(OkResponse::ok()))
}
