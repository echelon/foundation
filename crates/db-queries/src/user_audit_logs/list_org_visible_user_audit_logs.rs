use ids::{UserAuditLogId, UserId};
use sqlx::PgExecutor;

use super::UserAuditLog;

/// A member's audit trail AS AN ORG SEES IT: hard-filtered to `is_org_visible = true`
/// (audit_logs.md §5 — user-private rows like `session_revoked` never leave this query).
/// Newest first, keyset-paginated by row id. Uses the partial org-visible index.
pub async fn list_org_visible_user_audit_logs<'e, E: PgExecutor<'e>>(
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
         AND is_org_visible
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
