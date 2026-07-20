use ids::{AccountAuditLogId, AccountId, UserId};
use sqlx::PgExecutor;

use super::AccountAuditLog;

#[derive(Debug, Clone)]
pub struct NewAccountAuditLog<'a> {
  pub account_id: &'a AccountId,
  pub actor_user_id: Option<&'a UserId>,
  /// The member the event is about (e.g. the added/removed user), if any.
  pub user_id: Option<&'a UserId>,
  pub event: &'a str,
  pub target_type: Option<&'a str>,
  pub target_id: Option<&'a str>,
  pub data: Option<serde_json::Value>,
  pub ip: Option<&'a str>,
}

pub async fn create_account_audit_log<'e, E: PgExecutor<'e>>(
  exec: E,
  new: NewAccountAuditLog<'_>,
) -> sqlx::Result<AccountAuditLog> {
  let id = AccountAuditLogId::generate();
  let actor = new.actor_user_id.map(|u| u.as_str());
  let about = new.user_id.map(|u| u.as_str());
  sqlx::query_as!(
    AccountAuditLog,
    r#"INSERT INTO account_audit_logs
         (id, account_id, actor_user_id, user_id, event, target_type, target_id, data, ip)
       VALUES ($1, $2, $3, $4, $5, $6, $7, $8, ($9::text)::inet)
       RETURNING id AS "id: AccountAuditLogId", account_id AS "account_id: AccountId",
                 actor_user_id AS "actor_user_id?: UserId", user_id AS "user_id?: UserId",
                 event, target_type, target_id,
                 data AS "data?: serde_json::Value",
                 ip::text AS "ip?: String", created_at"#,
    id.as_str(),
    new.account_id.as_str(),
    actor,
    about,
    new.event,
    new.target_type,
    new.target_id,
    new.data,
    new.ip,
  )
  .fetch_one(exec)
  .await
}
