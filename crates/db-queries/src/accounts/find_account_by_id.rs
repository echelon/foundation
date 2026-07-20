use ids::AccountId;
use sqlx::PgExecutor;

use super::Account;

pub async fn find_account_by_id<'e, E: PgExecutor<'e>>(exec: E, id: &AccountId) -> sqlx::Result<Option<Account>> {
  sqlx::query_as!(
    Account,
    r#"SELECT id AS "id: AccountId", name, business_name, placeholder_business_name, status, created_at
       FROM accounts
       WHERE id = $1 AND deleted_at IS NULL"#,
    id.as_str(),
  )
  .fetch_optional(exec)
  .await
}
