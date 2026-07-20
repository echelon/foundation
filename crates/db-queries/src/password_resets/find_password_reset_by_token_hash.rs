use sqlx::PgExecutor;

use super::PasswordReset;

/// Resolve a reset from the emailed token's sha256. The caller checks unexpired + unused before
/// consuming.
pub async fn find_password_reset_by_token_hash<'e, E: PgExecutor<'e>>(
  exec: E,
  token_hash: &str,
) -> sqlx::Result<Option<PasswordReset>> {
  sqlx::query_as!(
    PasswordReset,
    r#"SELECT id AS "id: ids::PasswordResetId", user_id AS "user_id: ids::UserId",
              token_hash, created_at, expires_at, used_at
       FROM password_resets
       WHERE token_hash = $1"#,
    token_hash,
  )
  .fetch_optional(exec)
  .await
}
