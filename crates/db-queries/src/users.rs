//! `users` queries (accounts.md §2).

use chrono::{DateTime, Utc};
use sqlx::PgExecutor;

use crate::rows::User;

/// Columns selected for a [`User`] — never `sid`.
const COLS: &str = "id, email, email_clean, email_original, email_verified_at, password_hash, \
                    session_version, display_name, is_staff, created_at";

/// Fields for creating a user. `email` is the canonicalized (trim + lowercase) login key;
/// `email_clean` is the aggressively-canonicalized abuse signal; `email_original` is verbatim.
#[derive(Debug, Clone)]
pub struct NewUser {
  pub email: String,
  pub email_clean: String,
  pub email_original: String,
  pub password_hash: Option<String>,
  pub display_name: Option<String>,
  /// Set for SSO signups (provider asserts the email); null for email/password at launch.
  pub email_verified_at: Option<DateTime<Utc>>,
}

pub async fn create_user<'e, E: PgExecutor<'e>>(exec: E, new: NewUser) -> sqlx::Result<User> {
  let id = ids::new_id(ids::prefix::USER);
  let sql = format!(
    "INSERT INTO users (id, email, email_clean, email_original, password_hash, display_name, email_verified_at) \
     VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING {COLS}"
  );
  sqlx::query_as::<_, User>(&sql)
    .bind(id)
    .bind(new.email)
    .bind(new.email_clean)
    .bind(new.email_original)
    .bind(new.password_hash)
    .bind(new.display_name)
    .bind(new.email_verified_at)
    .fetch_one(exec)
    .await
}

pub async fn find_by_email<'e, E: PgExecutor<'e>>(exec: E, email: &str) -> sqlx::Result<Option<User>> {
  let sql = format!("SELECT {COLS} FROM users WHERE email = $1 AND deleted_at IS NULL");
  sqlx::query_as::<_, User>(&sql).bind(email).fetch_optional(exec).await
}

pub async fn find_by_id<'e, E: PgExecutor<'e>>(exec: E, id: &str) -> sqlx::Result<Option<User>> {
  let sql = format!("SELECT {COLS} FROM users WHERE id = $1 AND deleted_at IS NULL");
  sqlx::query_as::<_, User>(&sql).bind(id).fetch_optional(exec).await
}

/// Set/replace the password hash and bump `password_version` (a record, not the logout clock —
/// auth.md §3). Returns the number of rows updated.
pub async fn set_password_hash<'e, E: PgExecutor<'e>>(exec: E, user_id: &str, hash: &str) -> sqlx::Result<u64> {
  let r = sqlx::query(
    "UPDATE users SET password_hash = $2, password_version = password_version + 1, updated_at = now() WHERE id = $1",
  )
  .bind(user_id)
  .bind(hash)
  .execute(exec)
  .await?;
  Ok(r.rows_affected())
}

/// Bump `session_version` — the O(1) "log out all sessions" vector clock (auth.md §3).
pub async fn bump_session_version<'e, E: PgExecutor<'e>>(exec: E, user_id: &str) -> sqlx::Result<i32> {
  let sv: i32 = sqlx::query_scalar(
    "UPDATE users SET session_version = session_version + 1, updated_at = now() WHERE id = $1 RETURNING session_version",
  )
  .bind(user_id)
  .fetch_one(exec)
  .await?;
  Ok(sv)
}
