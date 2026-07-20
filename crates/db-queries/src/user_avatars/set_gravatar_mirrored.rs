use ids::{UserAvatarFileId, UserId};
use sqlx::PgExecutor;

/// Record a successful gravatar mirror (gravatar-job): link the mirrored file, stamp the probe,
/// and promote the display `source` to `gravatar` — unless the user has a custom avatar or has
/// dismissed gravatars (accounts.md §2 precedence). Returns rows affected.
pub async fn set_gravatar_mirrored<'e, E: PgExecutor<'e>>(
  exec: E,
  user_id: &UserId,
  file_id: &UserAvatarFileId,
) -> sqlx::Result<u64> {
  let result = sqlx::query!(
    r#"UPDATE user_avatars
       SET gravatar_file_id = $2,
           gravatar_checked_at = now(),
           source = CASE
                      WHEN source = 'custom' OR is_gravatar_dismissed THEN source
                      ELSE 'gravatar'
                    END,
           updated_at = now()
       WHERE user_id = $1"#,
    user_id.as_str(),
    file_id.as_str(),
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected())
}
