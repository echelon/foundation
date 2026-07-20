use ids::AccountId;
use sqlx::PgExecutor;

use super::Account;

/// Set (or clear — `None` falls back to the placeholder in clients) the business name.
/// Returns the updated row, or `None` for an unknown/deleted account.
pub async fn set_business_name<'e, E: PgExecutor<'e>>(
  exec: E,
  id: &AccountId,
  business_name: Option<&str>,
) -> sqlx::Result<Option<Account>> {
  sqlx::query_as!(
    Account,
    r#"UPDATE accounts SET business_name = $2
       WHERE id = $1 AND deleted_at IS NULL
       RETURNING id AS "id: AccountId", name, business_name, placeholder_business_name, status, created_at"#,
    id.as_str(),
    business_name,
  )
  .fetch_optional(exec)
  .await
}
