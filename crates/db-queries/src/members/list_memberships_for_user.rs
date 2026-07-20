use ids::UserId;
use sqlx::PgExecutor;

use super::AccountMember;

/// Every account the user belongs to (with their role), newest first.
pub async fn list_memberships_for_user<'e, E: PgExecutor<'e>>(
  exec: E,
  user_id: &UserId,
) -> sqlx::Result<Vec<AccountMember>> {
  sqlx::query_as!(
    AccountMember,
    r#"SELECT id AS "id: ids::AccountMemberId", account_id AS "account_id: ids::AccountId",
              user_id AS "user_id: UserId", role, created_at
       FROM account_members
       WHERE user_id = $1
       ORDER BY sid DESC"#,
    user_id.as_str(),
  )
  .fetch_all(exec)
  .await
}
