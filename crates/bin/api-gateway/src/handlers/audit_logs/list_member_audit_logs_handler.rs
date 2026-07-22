//! `GET /v1/accounts/{account_id}/members/{user_id}/audit_logs` — a member's trail AS THE ORG
//! SEES IT: hard-filtered to `is_org_visible = true`. Admin/owner only; target must be a member.

use axum::extract::{Path, Query, State};
use axum::Json;

use db_queries::{members, user_audit_logs};
use ids::{AccountId, UserAuditLogId, UserId};

use super::types::{PageParams, UserAuditLogsPage};
use crate::authz::require_account_role;
use crate::error::{ApiError, ApiResult};
use crate::middleware::AuthContext;
use crate::state::AppState;

#[utoipa::path(
  get, path = "/v1/accounts/{account_id}/members/{user_id}/audit_logs",
  tag = "audit-logs",
  security(("session_cookie" = [])),
  params(
    ("account_id" = String, Path, description = "Account id (`acct_…`)"),
    ("user_id" = String, Path, description = "The member's user id (`user_…`)"),
    PageParams
  ),
  responses(
    (status = 200, description = "The member's org-visible trail, newest first", body = UserAuditLogsPage),
    (status = 400, description = "Invalid id / pagination cursor"),
    (status = 404, description = "Not admin/owner, or the target is not a member")
  )
)]
pub async fn list_member_audit_logs_handler(
  State(state): State<AppState>,
  auth: AuthContext,
  Path((account_id, user_id)): Path<(String, String)>,
  Query(page): Query<PageParams>,
) -> ApiResult<Json<UserAuditLogsPage>> {
  let account_id: AccountId = account_id.parse().map_err(|_| ApiError::BadRequest("invalid account id".into()))?;
  let target_user_id: UserId = user_id.parse().map_err(|_| ApiError::BadRequest("invalid user id".into()))?;
  require_account_role(&state.pool, &auth.user.id, &account_id, enums::MemberRole::ADMINS).await?;
  members::find_membership(&state.pool, &account_id, &target_user_id)
    .await?
    .ok_or_else(|| ApiError::NotFound("that user is not a member of this organization".into()))?;

  let before = page.cursor::<UserAuditLogId>()?;
  let limit = page.limit();
  let rows =
    user_audit_logs::list_org_visible_user_audit_logs(&state.pool, &target_user_id, before.as_ref(), limit).await?;
  Ok(Json(UserAuditLogsPage::from_rows(rows, limit)))
}
