use ids::AccountId;
use sqlx::PgExecutor;

use super::AccountAuditLog;

/// An account's resource audit trail, newest first (audit_logs.md §5 `GET /v1/account/audit_logs`).
pub async fn list_account_audit_logs<'e, E: PgExecutor<'e>>(
  exec: E,
  account_id: &AccountId,
  limit: i64,
) -> sqlx::Result<Vec<AccountAuditLog>> {
  sqlx::query_as!(
    AccountAuditLog,
    r#"SELECT id AS "id: ids::AccountAuditLogId", account_id AS "account_id: AccountId",
              actor_user_id AS "actor_user_id?: ids::UserId", user_id AS "user_id?: ids::UserId",
              event, target_type, target_id,
              data AS "data?: serde_json::Value",
              ip::text AS "ip?: String", created_at
       FROM account_audit_logs
       WHERE account_id = $1
       ORDER BY created_at DESC
       LIMIT $2"#,
    account_id.as_str(),
    limit,
  )
  .fetch_all(exec)
  .await
}
