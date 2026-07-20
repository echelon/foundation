//! Audit-log read handlers (audit_logs.md §5). Three views, all keyset-paginated (pass the
//! last row's `id` back as `?before=` for the next page):
//!
//!   - `GET /v1/user/audit_logs` — the caller's OWN identity trail (private rows included).
//!   - `GET /v1/accounts/{id}/audit_logs` — the org's resource trail. Admin/owner only.
//!   - `GET /v1/accounts/{id}/members/{user_id}/audit_logs` — a member's trail AS THE ORG SEES
//!     IT: hard-filtered to `is_org_visible = true`. Admin/owner only; target must be a member.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use db_queries::{account_audit_logs, members, user_audit_logs, AccountAuditLog, UserAuditLog};
use ids::{AccountAuditLogId, AccountId, UserAuditLogId, UserId};

use crate::authz::require_account_role;
use crate::error::{ApiError, ApiResult};
use crate::middleware::AuthContext;
use crate::state::AppState;

const DEFAULT_LIMIT: i64 = 25;
const MAX_LIMIT: i64 = 100;

#[derive(Deserialize)]
pub struct PageParams {
  /// Row id from the previous page's last item.
  pub before: Option<String>,
  pub limit: Option<i64>,
}

impl PageParams {
  fn limit(&self) -> i64 {
    self.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT)
  }
}

/// `GET /v1/user/audit_logs?before&limit`
pub async fn list_my_audit_logs(
  State(state): State<AppState>,
  auth: AuthContext,
  Query(page): Query<PageParams>,
) -> ApiResult<Json<Value>> {
  let before = parse_cursor::<UserAuditLogId>(page.before.as_deref())?;
  let limit = page.limit();
  let rows = user_audit_logs::list_user_audit_logs(&state.pool, &auth.user.id, before.as_ref(), limit).await?;
  Ok(Json(paginate(rows.len() as i64 == limit, rows.iter().map(user_row).collect(), rows.last().map(|r| r.id.to_string()))))
}

/// `GET /v1/accounts/{account_id}/audit_logs?before&limit` — admin/owner only.
pub async fn list_account_audit_logs(
  State(state): State<AppState>,
  auth: AuthContext,
  Path(account_id): Path<String>,
  Query(page): Query<PageParams>,
) -> ApiResult<Json<Value>> {
  let account_id: AccountId = account_id.parse().map_err(|_| ApiError::BadRequest("invalid account id".into()))?;
  require_account_role(&state.pool, &auth.user.id, &account_id, &["owner", "admin"]).await?;

  let before = parse_cursor::<AccountAuditLogId>(page.before.as_deref())?;
  let limit = page.limit();
  let rows = account_audit_logs::list_account_audit_logs(&state.pool, &account_id, before.as_ref(), limit).await?;
  Ok(Json(paginate(rows.len() as i64 == limit, rows.iter().map(account_row).collect(), rows.last().map(|r| r.id.to_string()))))
}

/// `GET /v1/accounts/{account_id}/members/{user_id}/audit_logs?before&limit` — a member's
/// ORG-VISIBLE trail. Admin/owner only; 404 unless the target is a current member.
pub async fn list_member_audit_logs(
  State(state): State<AppState>,
  auth: AuthContext,
  Path((account_id, user_id)): Path<(String, String)>,
  Query(page): Query<PageParams>,
) -> ApiResult<Json<Value>> {
  let account_id: AccountId = account_id.parse().map_err(|_| ApiError::BadRequest("invalid account id".into()))?;
  let target_user_id: UserId = user_id.parse().map_err(|_| ApiError::BadRequest("invalid user id".into()))?;
  require_account_role(&state.pool, &auth.user.id, &account_id, &["owner", "admin"]).await?;
  members::find_membership(&state.pool, &account_id, &target_user_id)
    .await?
    .ok_or_else(|| ApiError::NotFound("that user is not a member of this organization".into()))?;

  let before = parse_cursor::<UserAuditLogId>(page.before.as_deref())?;
  let limit = page.limit();
  let rows =
    user_audit_logs::list_org_visible_user_audit_logs(&state.pool, &target_user_id, before.as_ref(), limit).await?;
  Ok(Json(paginate(rows.len() as i64 == limit, rows.iter().map(user_row).collect(), rows.last().map(|r| r.id.to_string()))))
}

/* ── helpers ── */

fn parse_cursor<T: std::str::FromStr>(raw: Option<&str>) -> ApiResult<Option<T>> {
  match raw {
    None => Ok(None),
    Some(s) => s.parse::<T>().map(Some).map_err(|_| ApiError::BadRequest("invalid pagination cursor".into())),
  }
}

/// The page envelope: `next_before` is present only when the page was full (there may be more).
fn paginate(full_page: bool, items: Vec<Value>, last_id: Option<String>) -> Value {
  json!({
    "items": items,
    "next_before": if full_page { last_id } else { None },
  })
}

fn user_row(r: &UserAuditLog) -> Value {
  json!({
    "id": r.id,
    "event": r.event,
    "actor_user_id": r.actor_user_id,
    "target_type": r.target_type,
    "target_id": r.target_id,
    "data": r.data,
    "is_org_visible": r.is_org_visible,
    "ip": r.ip,
    "created_at": r.created_at,
  })
}

fn account_row(r: &AccountAuditLog) -> Value {
  json!({
    "id": r.id,
    "event": r.event,
    "actor_user_id": r.actor_user_id,
    "user_id": r.user_id,
    "target_type": r.target_type,
    "target_id": r.target_id,
    "data": r.data,
    "ip": r.ip,
    "created_at": r.created_at,
  })
}
