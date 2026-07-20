use ids::UserId;
use sqlx::PgExecutor;

/// Re-derive all three email tiers on a user and **reset the verified flag** (accounts.md §2 email
/// change, step 3). The caller performs this inside the email-change transaction alongside the
/// `email_changes` row, the audit event, verification-token invalidation, and the avatar reset.
/// Returns rows affected.
pub async fn update_email<'e, E: PgExecutor<'e>>(
  exec: E,
  user_id: &UserId,
  email: &str,
  email_clean: &str,
  email_original: &str,
) -> sqlx::Result<u64> {
  let result = sqlx::query!(
    r#"UPDATE users
       SET email = $2, email_clean = $3, email_original = $4,
           email_verified_at = NULL, updated_at = now()
       WHERE id = $1"#,
    user_id.as_str(),
    email,
    email_clean,
    email_original,
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected())
}
