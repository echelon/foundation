use ids::UserId;
use sqlx::PgExecutor;

use super::UserAuditLog;

/// A user's own identity audit trail, newest first (audit_logs.md §5 `GET /v1/user/audit_logs`).
pub async fn list_user_audit_logs<'e, E: PgExecutor<'e>>(
  exec: E,
  user_id: &UserId,
  limit: i64,
) -> sqlx::Result<Vec<UserAuditLog>> {
  sqlx::query_as!(
    UserAuditLog,
    r#"SELECT id AS "id: ids::UserAuditLogId", user_id AS "user_id: UserId",
              actor_user_id AS "actor_user_id?: UserId",
              event, target_type, target_id,
              data AS "data?: serde_json::Value", is_org_visible,
              ip::text AS "ip?: String", created_at
       FROM user_audit_logs
       WHERE user_id = $1
       ORDER BY created_at DESC
       LIMIT $2"#,
    user_id.as_str(),
    limit,
  )
  .fetch_all(exec)
  .await
}
