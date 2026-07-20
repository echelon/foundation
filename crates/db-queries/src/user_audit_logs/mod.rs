//! `user_audit_logs` table (audit_logs.md §1) — user identity events (password, email, SSO,
//! sessions). Written in the SAME transaction as the mutation it records. `is_org_visible` is the
//! `AuditEvent::is_org_visible` verdict, consulted only here (identity events don't fan out to the
//! account log; org reach is this flag).

use chrono::{DateTime, Utc};
use ids::{UserAuditLogId, UserId};

mod create_user_audit_log;
mod list_user_audit_logs;

pub use create_user_audit_log::{create_user_audit_log, NewUserAuditLog};
pub use list_user_audit_logs::list_user_audit_logs;

#[derive(Debug, Clone)]
pub struct UserAuditLog {
  pub id: UserAuditLogId,
  pub user_id: UserId,
  pub actor_user_id: Option<UserId>,
  pub event: String,
  pub target_type: Option<String>,
  pub target_id: Option<String>,
  pub data: Option<serde_json::Value>,
  pub is_org_visible: bool,
  pub ip: Option<String>,
  pub created_at: DateTime<Utc>,
}
