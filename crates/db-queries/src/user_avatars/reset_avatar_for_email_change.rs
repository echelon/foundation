use ids::UserId;
use sqlx::PgExecutor;

/// Re-run the avatar pipeline after an email change (accounts.md §2 step 5): recompute the
/// `gravatar_hash`, refresh the letter monogram, drop the old address's mirror, and reset
/// `gravatar_checked_at = NULL` so the gravatar-job re-probes. The display PREFERENCE is
/// untouched — a `gravatar` preference shows the monogram until the new mirror lands; a
/// `custom` preference is unaffected. Returns rows affected.
pub async fn reset_avatar_for_email_change<'e, E: PgExecutor<'e>>(
  exec: E,
  user_id: &UserId,
  gravatar_hash: &str,
  email_first_letter: Option<&str>,
  email_last_letter: Option<&str>,
) -> sqlx::Result<u64> {
  let result = sqlx::query!(
    r#"UPDATE user_avatars
       SET gravatar_hash = $2,
           gravatar_checked_at = NULL,
           gravatar_file_id = NULL,
           email_first_letter = $3,
           email_last_letter = $4,
           updated_at = now()
       WHERE user_id = $1"#,
    user_id.as_str(),
    gravatar_hash,
    email_first_letter,
    email_last_letter,
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected())
}
