//! Shared audit-log wire types: the pagination query and the typed page envelopes.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use db_queries::{AccountAuditLog, UserAuditLog};
use ids::{AccountAuditLogId, UserAuditLogId, UserId};

use crate::error::{ApiError, ApiResult};

const DEFAULT_LIMIT: i64 = 25;
const MAX_LIMIT: i64 = 100;

/// Keyset pagination: pass the previous page's last row id back as `before`.
#[derive(Debug, Deserialize, IntoParams)]
pub struct PageParams {
  /// Row id from the previous page's last item.
  pub before: Option<String>,
  /// Page size, 1–100 (default 25).
  pub limit: Option<i64>,
}

impl PageParams {
  pub fn limit(&self) -> i64 {
    self.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT)
  }

  /// The `before` cursor parsed to the view's typed row id.
  pub fn cursor<T: std::str::FromStr>(&self) -> ApiResult<Option<T>> {
    match self.before.as_deref() {
      None => Ok(None),
      Some(s) => s.parse::<T>().map(Some).map_err(|_| ApiError::BadRequest("invalid pagination cursor".into())),
    }
  }
}

/// One row of a user's identity trail.
#[derive(Debug, Serialize, ToSchema)]
pub struct UserAuditLogItem {
  #[schema(value_type = String, example = "uaudit_01j9…")]
  pub id: UserAuditLogId,
  pub event: String,
  #[schema(value_type = Option<String>, example = "user_01j9…")]
  pub actor_user_id: Option<UserId>,
  pub target_type: Option<String>,
  pub target_id: Option<String>,
  /// Event-specific payload (freeform).
  #[schema(value_type = Option<Object>)]
  pub data: Option<serde_json::Value>,
  pub is_org_visible: bool,
  pub ip: Option<String>,
  pub created_at: DateTime<Utc>,
}

impl From<UserAuditLog> for UserAuditLogItem {
  fn from(r: UserAuditLog) -> Self {
    Self {
      id: r.id,
      event: r.event,
      actor_user_id: r.actor_user_id,
      target_type: r.target_type,
      target_id: r.target_id,
      data: r.data,
      is_org_visible: r.is_org_visible,
      ip: r.ip,
      created_at: r.created_at,
    }
  }
}

/// One row of an org's resource trail.
#[derive(Debug, Serialize, ToSchema)]
pub struct AccountAuditLogItem {
  #[schema(value_type = String, example = "aaudit_01j9…")]
  pub id: AccountAuditLogId,
  pub event: String,
  #[schema(value_type = Option<String>, example = "user_01j9…")]
  pub actor_user_id: Option<UserId>,
  #[schema(value_type = Option<String>, example = "user_01j9…")]
  pub user_id: Option<UserId>,
  pub target_type: Option<String>,
  pub target_id: Option<String>,
  /// Event-specific payload (freeform).
  #[schema(value_type = Option<Object>)]
  pub data: Option<serde_json::Value>,
  pub ip: Option<String>,
  pub created_at: DateTime<Utc>,
}

impl From<AccountAuditLog> for AccountAuditLogItem {
  fn from(r: AccountAuditLog) -> Self {
    Self {
      id: r.id,
      event: r.event,
      actor_user_id: r.actor_user_id,
      user_id: r.user_id,
      target_type: r.target_type,
      target_id: r.target_id,
      data: r.data,
      ip: r.ip,
      created_at: r.created_at,
    }
  }
}

/// A page of user-trail rows. `next_before` is present only when the page was full (there may
/// be more) — pass it back as `?before=` for the next page.
#[derive(Debug, Serialize, ToSchema)]
pub struct UserAuditLogsPage {
  pub items: Vec<UserAuditLogItem>,
  pub next_before: Option<String>,
}

impl UserAuditLogsPage {
  pub fn from_rows(rows: Vec<UserAuditLog>, limit: i64) -> Self {
    let full_page = rows.len() as i64 == limit;
    let next_before = full_page.then(|| rows.last().map(|r| r.id.to_string())).flatten();
    Self { items: rows.into_iter().map(Into::into).collect(), next_before }
  }
}

/// A page of account-trail rows. Same cursor contract as [`UserAuditLogsPage`].
#[derive(Debug, Serialize, ToSchema)]
pub struct AccountAuditLogsPage {
  pub items: Vec<AccountAuditLogItem>,
  pub next_before: Option<String>,
}

impl AccountAuditLogsPage {
  pub fn from_rows(rows: Vec<AccountAuditLog>, limit: i64) -> Self {
    let full_page = rows.len() as i64 == limit;
    let next_before = full_page.then(|| rows.last().map(|r| r.id.to_string())).flatten();
    Self { items: rows.into_iter().map(Into::into).collect(), next_before }
  }
}
