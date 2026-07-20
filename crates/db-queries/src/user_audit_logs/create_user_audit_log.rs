use enums::AuditEvent;
use ids::{UserAuditLogId, UserId};
use sqlx::PgExecutor;

use super::UserAuditLog;

/// A user-audit event. `is_org_visible` is derived from the event (`AuditEvent::is_org_visible`
/// — audit_logs.md §3), never passed. `actor_user_id` is the user who caused it (usually the
/// subject; NULL for a system action).
#[derive(Debug, Clone)]
pub struct NewUserAuditLog<'a> {
  pub user_id: &'a UserId,
  pub actor_user_id: Option<&'a UserId>,
  pub event: AuditEvent,
  pub target_type: Option<&'a str>,
  pub target_id: Option<&'a str>,
  pub data: Option<serde_json::Value>,
  pub ip: Option<&'a str>,
}

pub async fn create_user_audit_log<'e, E: PgExecutor<'e>>(
  exec: E,
  new: NewUserAuditLog<'_>,
) -> sqlx::Result<UserAuditLog> {
  let id = UserAuditLogId::generate();
  let actor = new.actor_user_id.map(|u| u.as_str());
  sqlx::query_as!(
    UserAuditLog,
    r#"INSERT INTO user_audit_logs
         (id, user_id, actor_user_id, event, target_type, target_id, data, is_org_visible, ip)
       VALUES ($1, $2, $3, $4, $5, $6, $7, $8, ($9::text)::inet)
       RETURNING id AS "id: UserAuditLogId", user_id AS "user_id: UserId",
                 actor_user_id AS "actor_user_id?: UserId",
                 event, target_type, target_id,
                 data AS "data?: serde_json::Value", is_org_visible,
                 host(ip) AS "ip?: String", created_at"#,
    id.as_str(),
    new.user_id.as_str(),
    actor,
    new.event.as_str(),
    new.target_type,
    new.target_id,
    new.data,
    new.event.is_org_visible(),
    new.ip,
  )
  .fetch_one(exec)
  .await
}
