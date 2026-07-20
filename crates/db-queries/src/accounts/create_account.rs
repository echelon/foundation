use ids::AccountId;
use sqlx::PgExecutor;

use super::Account;

#[derive(Debug, Clone)]
pub struct NewAccount<'a> {
  pub name: Option<&'a str>,
  /// The random "Adjective Noun" shown until an owner/admin sets `business_name`.
  pub placeholder_business_name: &'a str,
}

/// Create an account (status defaults to `active`).
pub async fn create_account<'e, E: PgExecutor<'e>>(exec: E, new: NewAccount<'_>) -> sqlx::Result<Account> {
  let id = AccountId::generate();
  sqlx::query_as!(
    Account,
    r#"INSERT INTO accounts (id, name, placeholder_business_name, status)
       VALUES ($1, $2, $3, 'active')
       RETURNING id AS "id: AccountId", name, business_name, placeholder_business_name, status, created_at"#,
    id.as_str(),
    new.name,
    new.placeholder_business_name,
  )
  .fetch_one(exec)
  .await
}
