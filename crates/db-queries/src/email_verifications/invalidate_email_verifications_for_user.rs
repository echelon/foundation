use ids::UserId;
use sqlx::PgExecutor;

/// Invalidate all outstanding (unvalidated, unexpired) verifications for a user by expiring them
/// now (accounts.md §2 email-change step 4). Returns the number invalidated.
pub async fn invalidate_email_verifications_for_user<'e, E: PgExecutor<'e>>(
  exec: E,
  user_id: &UserId,
) -> sqlx::Result<u64> {
  let result = sqlx::query!(
    r#"UPDATE email_verifications SET expires_at = now()
       WHERE user_id = $1 AND validated_at IS NULL AND expires_at > now()"#,
    user_id.as_str(),
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected())
}
