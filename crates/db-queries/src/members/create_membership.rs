use ids::{AccountId, AccountMemberId, UserId};
use sqlx::PgExecutor;

use super::AccountMember;

/// Add a membership. `role` is CHECK-constrained to `owner`/`admin`/`member`; a second `owner`
/// on the same account is rejected by the partial unique index.
pub async fn create_membership<'e, E: PgExecutor<'e>>(
  exec: E,
  account_id: &AccountId,
  user_id: &UserId,
  role: &str,
) -> sqlx::Result<AccountMember> {
  let id = AccountMemberId::generate();
  sqlx::query_as!(
    AccountMember,
    r#"INSERT INTO account_members (id, account_id, user_id, role)
       VALUES ($1, $2, $3, $4)
       RETURNING id AS "id: AccountMemberId", account_id AS "account_id: AccountId",
                 user_id AS "user_id: UserId", role, created_at"#,
    id.as_str(),
    account_id.as_str(),
    user_id.as_str(),
    role,
  )
  .fetch_one(exec)
  .await
}
