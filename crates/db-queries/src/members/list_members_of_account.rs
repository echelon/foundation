use chrono::{DateTime, Utc};
use ids::{AccountId, UserId};
use sqlx::PgExecutor;

/// A member joined with their user row — what the organization members page renders.
#[derive(Debug, Clone)]
pub struct AccountMemberDetail {
  pub user_id: UserId,
  pub display_name: Option<String>,
  pub email: String,
  pub role: String,
  pub created_at: DateTime<Utc>,
}

/// Everyone in the account (live users only) — oldest first, so the owner leads the roster.
pub async fn list_members_of_account<'e, E: PgExecutor<'e>>(
  exec: E,
  account_id: &AccountId,
) -> sqlx::Result<Vec<AccountMemberDetail>> {
  sqlx::query_as!(
    AccountMemberDetail,
    r#"SELECT m.user_id AS "user_id: UserId", u.display_name, u.email, m.role, m.created_at
       FROM account_members m
       JOIN users u ON u.id = m.user_id
       WHERE m.account_id = $1 AND u.deleted_at IS NULL
       ORDER BY m.sid ASC"#,
    account_id.as_str(),
  )
  .fetch_all(exec)
  .await
}
