use ids::{UserAuditLogId, UserId};
use sqlx::PgExecutor;

use super::UserAuditLog;

/// A user's own identity audit trail, newest first, keyset-paginated: pass the last row's `id`
/// as `before` to get the next page (audit_logs.md §5 `GET /v1/user/audit_logs`). The cursor
/// resolves to the internal `sid` — never exposed — so pagination is stable under inserts.
pub async fn list_user_audit_logs<'e, E: PgExecutor<'e>>(
  exec: E,
  user_id: &UserId,
  before: Option<&UserAuditLogId>,
  limit: i64,
) -> sqlx::Result<Vec<UserAuditLog>> {
  let before = before.map(|b| b.as_str());
  sqlx::query_as!(
    UserAuditLog,
    r#"SELECT id AS "id: ids::UserAuditLogId", user_id AS "user_id: UserId",
              actor_user_id AS "actor_user_id?: UserId",
              event, target_type, target_id,
              data AS "data?: serde_json::Value", is_org_visible,
              ip::text AS "ip?: String", created_at
       FROM user_audit_logs
       WHERE user_id = $1
         AND ($2::text IS NULL OR sid < (SELECT sid FROM user_audit_logs WHERE id = $2))
       ORDER BY sid DESC
       LIMIT $3"#,
    user_id.as_str(),
    before,
    limit,
  )
  .fetch_all(exec)
  .await
}
