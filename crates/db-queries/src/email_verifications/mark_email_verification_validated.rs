use ids::EmailVerificationId;
use sqlx::PgExecutor;

/// Consume a verification token: set `validated_at` if still open. Returns whether a row was
/// updated (0 → already used/expired).
pub async fn mark_email_verification_validated<'e, E: PgExecutor<'e>>(
  exec: E,
  id: &EmailVerificationId,
) -> sqlx::Result<bool> {
  let result = sqlx::query!(
    r#"UPDATE email_verifications SET validated_at = now()
       WHERE id = $1 AND validated_at IS NULL AND expires_at > now()"#,
    id.as_str(),
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected() > 0)
}
