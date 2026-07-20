use ids::{UserAvatarFileId, UserId};
use sqlx::PgExecutor;

/// Record a successful gravatar mirror (gravatar-job): link the mirrored file and stamp the
/// probe. Whether it DISPLAYS is the user's `preferred_source` (default `gravatar`, so new
/// users see it automatically; a `custom`/`letter` preference leaves it available-but-hidden).
/// Returns rows affected.
pub async fn set_gravatar_mirrored<'e, E: PgExecutor<'e>>(
  exec: E,
  user_id: &UserId,
  file_id: &UserAvatarFileId,
) -> sqlx::Result<u64> {
  let result = sqlx::query!(
    r#"UPDATE user_avatars
       SET gravatar_file_id = $2, gravatar_checked_at = now(), updated_at = now()
       WHERE user_id = $1"#,
    user_id.as_str(),
    file_id.as_str(),
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected())
}
