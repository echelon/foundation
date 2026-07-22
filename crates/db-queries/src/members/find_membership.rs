use ids::{AccountId, UserId};
use sqlx::PgExecutor;

use super::AccountMember;

pub async fn find_membership<'e, E: PgExecutor<'e>>(
  exec: E,
  account_id: &AccountId,
  user_id: &UserId,
) -> sqlx::Result<Option<AccountMember>> {
  sqlx::query_as!(
    AccountMember,
    r#"SELECT id AS "id: ids::AccountMemberId", account_id AS "account_id: AccountId",
              user_id AS "user_id: UserId", role AS "role: enums::MemberRole", created_at
       FROM account_members
       WHERE account_id = $1 AND user_id = $2"#,
    account_id.as_str(),
    user_id.as_str(),
  )
  .fetch_optional(exec)
  .await
}
