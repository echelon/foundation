//! `GET /v1/user/audit_logs` — the caller's OWN identity trail (private rows included).

use axum::extract::{Query, State};
use axum::Json;

use db_queries::user_audit_logs;
use ids::UserAuditLogId;

use super::types::{PageParams, UserAuditLogsPage};
use crate::error::ApiResult;
use crate::middleware::AuthContext;
use crate::state::AppState;

#[utoipa::path(
  get, path = "/v1/user/audit_logs",
  tag = "audit-logs",
  security(("session_cookie" = [])),
  params(PageParams),
  responses(
    (status = 200, description = "The caller's identity trail, newest first", body = UserAuditLogsPage),
    (status = 400, description = "Invalid pagination cursor")
  )
)]
pub async fn list_my_audit_logs_handler(
  State(state): State<AppState>,
  auth: AuthContext,
  Query(page): Query<PageParams>,
) -> ApiResult<Json<UserAuditLogsPage>> {
  let before = page.cursor::<UserAuditLogId>()?;
  let limit = page.limit();
  let rows = user_audit_logs::list_user_audit_logs(&state.pool, &auth.user.id, before.as_ref(), limit).await?;
  Ok(Json(UserAuditLogsPage::from_rows(rows, limit)))
}
