//! `accounts` queries (accounts.md §2).

use sqlx::PgExecutor;

use crate::rows::Account;

const COLS: &str = "id, name, status, created_at";

/// Create an account (status defaults to `active`). Returns the new row.
pub async fn create_account<'e, E: PgExecutor<'e>>(exec: E, name: Option<&str>) -> sqlx::Result<Account> {
  let id = ids::new_id(ids::prefix::ACCOUNT);
  let sql = format!("INSERT INTO accounts (id, name, status) VALUES ($1, $2, 'active') RETURNING {COLS}");
  sqlx::query_as::<_, Account>(&sql).bind(id).bind(name).fetch_one(exec).await
}

pub async fn find_by_id<'e, E: PgExecutor<'e>>(exec: E, id: &str) -> sqlx::Result<Option<Account>> {
  let sql = format!("SELECT {COLS} FROM accounts WHERE id = $1 AND deleted_at IS NULL");
  sqlx::query_as::<_, Account>(&sql).bind(id).fetch_optional(exec).await
}
