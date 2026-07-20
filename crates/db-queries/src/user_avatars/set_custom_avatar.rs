use ids::{UserAvatarFileId, UserId};
use sqlx::PgExecutor;

/// Set an uploaded custom avatar AND prefer it (uploading a photo means you want to see it).
/// The mirrored gravatar is untouched — switch back any time via `set_preferred_source`.
/// Returns rows affected.
pub async fn set_custom_avatar<'e, E: PgExecutor<'e>>(
  exec: E,
  user_id: &UserId,
  file_id: &UserAvatarFileId,
) -> sqlx::Result<u64> {
  let result = sqlx::query!(
    r#"UPDATE user_avatars
       SET custom_file_id = $2, preferred_source = 'custom', updated_at = now()
       WHERE user_id = $1"#,
    user_id.as_str(),
    file_id.as_str(),
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected())
}
