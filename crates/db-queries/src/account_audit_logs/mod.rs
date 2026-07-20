//! `account_audit_logs` table (audit_logs.md §1) — account resource events (membership, keys,
//! billing profile, limits). Written in the same transaction as the mutation. `account_id` scopes
//! the log; `user_id` is the member the event is about (if any); `actor_user_id` is who did it.

use chrono::{DateTime, Utc};
use ids::{AccountAuditLogId, AccountId, UserId};

mod create_account_audit_log;
mod list_account_audit_logs;

pub use create_account_audit_log::{create_account_audit_log, NewAccountAuditLog};
pub use list_account_audit_logs::list_account_audit_logs;

#[derive(Debug, Clone)]
pub struct AccountAuditLog {
  pub id: AccountAuditLogId,
  pub account_id: AccountId,
  pub actor_user_id: Option<UserId>,
  pub user_id: Option<UserId>,
  pub event: String,
  pub target_type: Option<String>,
  pub target_id: Option<String>,
  pub data: Option<serde_json::Value>,
  pub ip: Option<String>,
  pub created_at: DateTime<Utc>,
}
