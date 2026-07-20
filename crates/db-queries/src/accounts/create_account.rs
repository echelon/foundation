use ids::AccountId;
use sqlx::PgExecutor;

use super::Account;

/// Create an account (status defaults to `active`).
pub async fn create_account<'e, E: PgExecutor<'e>>(exec: E, name: Option<&str>) -> sqlx::Result<Account> {
  let id = AccountId::generate();
  sqlx::query_as!(
    Account,
    r#"INSERT INTO accounts (id, name, status)
       VALUES ($1, $2, 'active')
       RETURNING id AS "id: AccountId", name, status, created_at"#,
    id.as_str(),
    name,
  )
  .fetch_one(exec)
  .await
}
