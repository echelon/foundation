//! `account_members` queries (accounts.md §2). Exactly one `owner` per account (partial
//! unique index); roles gate authorization (auth.md §4).

use sqlx::PgExecutor;

use crate::rows::AccountMember;

const COLS: &str = "id, account_id, user_id, role, created_at";

/// Add a membership. `role` must be one of `owner`/`admin`/`member` (DB CHECK enforces it,
/// and the partial unique index rejects a second `owner`).
pub async fn create_membership<'e, E: PgExecutor<'e>>(
  exec: E,
  account_id: &str,
  user_id: &str,
  role: &str,
) -> sqlx::Result<AccountMember> {
  let id = ids::new_id(ids::prefix::ACCOUNT_MEMBER);
  let sql = format!(
    "INSERT INTO account_members (id, account_id, user_id, role) VALUES ($1, $2, $3, $4) RETURNING {COLS}"
  );
  sqlx::query_as::<_, AccountMember>(&sql)
    .bind(id)
    .bind(account_id)
    .bind(user_id)
    .bind(role)
    .fetch_one(exec)
    .await
}

pub async fn find_membership<'e, E: PgExecutor<'e>>(
  exec: E,
  account_id: &str,
  user_id: &str,
) -> sqlx::Result<Option<AccountMember>> {
  let sql = format!("SELECT {COLS} FROM account_members WHERE account_id = $1 AND user_id = $2");
  sqlx::query_as::<_, AccountMember>(&sql)
    .bind(account_id)
    .bind(user_id)
    .fetch_optional(exec)
    .await
}

/// Every account the user belongs to (with their role), newest first.
pub async fn list_for_user<'e, E: PgExecutor<'e>>(exec: E, user_id: &str) -> sqlx::Result<Vec<AccountMember>> {
  let sql = format!("SELECT {COLS} FROM account_members WHERE user_id = $1 ORDER BY sid DESC");
  sqlx::query_as::<_, AccountMember>(&sql).bind(user_id).fetch_all(exec).await
}
