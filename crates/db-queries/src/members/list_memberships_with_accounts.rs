use chrono::{DateTime, Utc};
use ids::{AccountId, UserId};
use sqlx::PgExecutor;

/// A membership joined with its account — what the org switcher renders.
#[derive(Debug, Clone)]
pub struct MembershipWithAccount {
  pub account_id: AccountId,
  pub account_name: Option<String>,
  pub role: String,
  pub created_at: DateTime<Utc>,
}

/// Every (live) account the user belongs to, with their role — oldest membership first, so the
/// signup-created account lists first and switcher order is stable.
pub async fn list_memberships_with_accounts<'e, E: PgExecutor<'e>>(
  exec: E,
  user_id: &UserId,
) -> sqlx::Result<Vec<MembershipWithAccount>> {
  sqlx::query_as!(
    MembershipWithAccount,
    r#"SELECT m.account_id AS "account_id: AccountId", a.name AS "account_name?: String",
              m.role, m.created_at
       FROM account_members m
       JOIN accounts a ON a.id = m.account_id
       WHERE m.user_id = $1 AND a.deleted_at IS NULL
       ORDER BY m.sid ASC"#,
    user_id.as_str(),
  )
  .fetch_all(exec)
  .await
}
