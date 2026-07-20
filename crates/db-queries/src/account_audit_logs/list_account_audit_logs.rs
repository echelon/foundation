use ids::{AccountAuditLogId, AccountId};
use sqlx::PgExecutor;

use super::AccountAuditLog;

/// An account's resource audit trail, newest first, keyset-paginated by row id
/// (audit_logs.md §5 `GET /v1/account/audit_logs` — admin/owner only, enforced by the caller).
pub async fn list_account_audit_logs<'e, E: PgExecutor<'e>>(
  exec: E,
  account_id: &AccountId,
  before: Option<&AccountAuditLogId>,
  limit: i64,
) -> sqlx::Result<Vec<AccountAuditLog>> {
  let before = before.map(|b| b.as_str());
  sqlx::query_as!(
    AccountAuditLog,
    r#"SELECT id AS "id: ids::AccountAuditLogId", account_id AS "account_id: AccountId",
              actor_user_id AS "actor_user_id?: ids::UserId", user_id AS "user_id?: ids::UserId",
              event, target_type, target_id,
              data AS "data?: serde_json::Value",
              ip::text AS "ip?: String", created_at
       FROM account_audit_logs
       WHERE account_id = $1
         AND ($2::text IS NULL OR sid < (SELECT sid FROM account_audit_logs WHERE id = $2))
       ORDER BY sid DESC
       LIMIT $3"#,
    account_id.as_str(),
    before,
    limit,
  )
  .fetch_all(exec)
  .await
}
