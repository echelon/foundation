//! `sessions` queries (accounts.md §2 / auth.md §1). The cookie is a signed JWT rollup of
//! state; the row is the revocation authority (looked up by `id` on every request).

use chrono::{DateTime, Utc};
use sqlx::PgExecutor;

use crate::rows::Session;

const COLS: &str = "id, user_id, impersonator_user_id, created_at, expires_at, revoked_at";

#[derive(Debug, Clone, Default)]
pub struct NewSession<'a> {
  pub user_id: &'a str,
  pub expires_at: DateTime<Utc>,
  pub ip: Option<&'a str>,
  pub user_agent: Option<&'a str>,
  pub device: Option<&'a str>,
  /// Set only for staff impersonation (auth.md §6).
  pub impersonator_user_id: Option<&'a str>,
}

pub async fn create_session<'e, E: PgExecutor<'e>>(exec: E, new: NewSession<'_>) -> sqlx::Result<Session> {
  let id = ids::new_id(ids::prefix::SESSION);
  let sql = format!(
    "INSERT INTO sessions (id, user_id, impersonator_user_id, ip, user_agent, device, expires_at) \
     VALUES ($1, $2, $3, $4::inet, $5, $6, $7) RETURNING {COLS}"
  );
  sqlx::query_as::<_, Session>(&sql)
    .bind(id)
    .bind(new.user_id)
    .bind(new.impersonator_user_id)
    .bind(new.ip)
    .bind(new.user_agent)
    .bind(new.device)
    .bind(new.expires_at)
    .fetch_one(exec)
    .await
}

pub async fn find_by_id<'e, E: PgExecutor<'e>>(exec: E, id: &str) -> sqlx::Result<Option<Session>> {
  let sql = format!("SELECT {COLS} FROM sessions WHERE id = $1");
  sqlx::query_as::<_, Session>(&sql).bind(id).fetch_optional(exec).await
}

/// Kill one session (writes `revoked_at`). Idempotent. Returns rows affected.
pub async fn revoke<'e, E: PgExecutor<'e>>(exec: E, id: &str) -> sqlx::Result<u64> {
  let r = sqlx::query("UPDATE sessions SET revoked_at = now() WHERE id = $1 AND revoked_at IS NULL")
    .bind(id)
    .execute(exec)
    .await?;
  Ok(r.rows_affected())
}

/// Live (non-revoked, non-expired) sessions for a user, newest first, excluding impersonation
/// sessions (auth.md §3).
pub async fn list_live_for_user<'e, E: PgExecutor<'e>>(exec: E, user_id: &str) -> sqlx::Result<Vec<Session>> {
  let sql = format!(
    "SELECT {COLS} FROM sessions \
     WHERE user_id = $1 AND revoked_at IS NULL AND expires_at > now() AND impersonator_user_id IS NULL \
     ORDER BY sid DESC"
  );
  sqlx::query_as::<_, Session>(&sql).bind(user_id).fetch_all(exec).await
}

/// Revoke every session for a user EXCEPT `keep_id` (the "log out others" write half; the
/// caller also bumps `users.session_version`). Returns rows affected.
pub async fn revoke_all_except<'e, E: PgExecutor<'e>>(exec: E, user_id: &str, keep_id: &str) -> sqlx::Result<u64> {
  let r = sqlx::query(
    "UPDATE sessions SET revoked_at = now() WHERE user_id = $1 AND id <> $2 AND revoked_at IS NULL",
  )
  .bind(user_id)
  .bind(keep_id)
  .execute(exec)
  .await?;
  Ok(r.rows_affected())
}
