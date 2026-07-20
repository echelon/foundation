//! `GET /v1/accounts/{account_id}/audit_logs` — the org's resource trail. Admin/owner only.

use axum::extract::{Path, Query, State};
use axum::Json;

use db_queries::account_audit_logs;
use ids::{AccountAuditLogId, AccountId};

use super::types::{AccountAuditLogsPage, PageParams};
use crate::authz::require_account_role;
use crate::error::{ApiError, ApiResult};
use crate::middleware::AuthContext;
use crate::state::AppState;

#[utoipa::path(
  get, path = "/v1/accounts/{account_id}/audit_logs",
  tag = "audit-logs",
  security(("session_cookie" = [])),
  params(
    ("account_id" = String, Path, description = "Account id (`acct_…`)"),
    PageParams
  ),
  responses(
    (status = 200, description = "The org's resource trail, newest first", body = AccountAuditLogsPage),
    (status = 400, description = "Invalid account id / pagination cursor"),
    (status = 404, description = "Not a member, or not admin/owner")
  )
)]
pub async fn list_account_audit_logs_handler(
  State(state): State<AppState>,
  auth: AuthContext,
  Path(account_id): Path<String>,
  Query(page): Query<PageParams>,
) -> ApiResult<Json<AccountAuditLogsPage>> {
  let account_id: AccountId = account_id.parse().map_err(|_| ApiError::BadRequest("invalid account id".into()))?;
  require_account_role(&state.pool, &auth.user.id, &account_id, &["owner", "admin"]).await?;

  let before = page.cursor::<AccountAuditLogId>()?;
  let limit = page.limit();
  let rows = account_audit_logs::list_account_audit_logs(&state.pool, &account_id, before.as_ref(), limit).await?;
  Ok(Json(AccountAuditLogsPage::from_rows(rows, limit)))
}
