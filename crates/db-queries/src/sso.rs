//! `user_sso_connections` queries (accounts.md §2). Resolve login on `(provider,
//! provider_user_id)` — NEVER on email (prevents takeover via a same-email SSO account).

use sqlx::PgExecutor;

use crate::rows::SsoConnection;

const COLS: &str = "id, user_id, provider, provider_user_id, provider_email, created_at, last_used_at";

#[derive(Debug, Clone)]
pub struct NewSsoConnection<'a> {
  pub user_id: &'a str,
  /// `google` | `github`.
  pub provider: &'a str,
  /// Stable provider id (`sub` / numeric github id) — never the email.
  pub provider_user_id: &'a str,
  pub provider_email: &'a str,
  pub provider_user_display_name: Option<&'a str>,
}

pub async fn create_connection<'e, E: PgExecutor<'e>>(exec: E, new: NewSsoConnection<'_>) -> sqlx::Result<SsoConnection> {
  let id = ids::new_id(ids::prefix::SSO_CONNECTION);
  let sql = format!(
    "INSERT INTO user_sso_connections \
       (id, user_id, provider, provider_user_id, provider_email, provider_user_display_name, last_used_at) \
     VALUES ($1, $2, $3, $4, $5, $6, now()) RETURNING {COLS}"
  );
  sqlx::query_as::<_, SsoConnection>(&sql)
    .bind(id)
    .bind(new.user_id)
    .bind(new.provider)
    .bind(new.provider_user_id)
    .bind(new.provider_email)
    .bind(new.provider_user_display_name)
    .fetch_one(exec)
    .await
}

pub async fn find_by_provider_sub<'e, E: PgExecutor<'e>>(
  exec: E,
  provider: &str,
  provider_user_id: &str,
) -> sqlx::Result<Option<SsoConnection>> {
  let sql = format!("SELECT {COLS} FROM user_sso_connections WHERE provider = $1 AND provider_user_id = $2");
  sqlx::query_as::<_, SsoConnection>(&sql)
    .bind(provider)
    .bind(provider_user_id)
    .fetch_optional(exec)
    .await
}

pub async fn touch_last_used<'e, E: PgExecutor<'e>>(exec: E, id: &str) -> sqlx::Result<u64> {
  let r = sqlx::query("UPDATE user_sso_connections SET last_used_at = now() WHERE id = $1")
    .bind(id)
    .execute(exec)
    .await?;
  Ok(r.rows_affected())
}

/// Every SSO connection on a user (for the login-methods list, accounts.md §6).
pub async fn list_for_user<'e, E: PgExecutor<'e>>(exec: E, user_id: &str) -> sqlx::Result<Vec<SsoConnection>> {
  let sql = format!("SELECT {COLS} FROM user_sso_connections WHERE user_id = $1 ORDER BY sid DESC");
  sqlx::query_as::<_, SsoConnection>(&sql).bind(user_id).fetch_all(exec).await
}
