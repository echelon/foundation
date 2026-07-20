use ids::PasswordResetId;
use sqlx::PgExecutor;

/// Consume a reset token: set `used_at` if still open (single-use). Returns whether a row was
/// updated (0 → already used or expired → caller rejects).
pub async fn mark_password_reset_used<'e, E: PgExecutor<'e>>(
  exec: E,
  id: &PasswordResetId,
) -> sqlx::Result<bool> {
  let result = sqlx::query!(
    r#"UPDATE password_resets SET used_at = now()
       WHERE id = $1 AND used_at IS NULL AND expires_at > now()"#,
    id.as_str(),
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected() > 0)
}
