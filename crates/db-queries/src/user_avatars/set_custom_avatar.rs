use ids::{UserAvatarFileId, UserId};
use sqlx::PgExecutor;

/// Set an uploaded custom avatar as the display source. Custom overrides gravatar but does NOT
/// discard the mirrored gravatar state (accounts.md §2). Returns rows affected.
pub async fn set_custom_avatar<'e, E: PgExecutor<'e>>(
  exec: E,
  user_id: &UserId,
  file_id: &UserAvatarFileId,
) -> sqlx::Result<u64> {
  let result = sqlx::query!(
    r#"UPDATE user_avatars
       SET custom_file_id = $2, source = 'custom', updated_at = now()
       WHERE user_id = $1"#,
    user_id.as_str(),
    file_id.as_str(),
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected())
}
