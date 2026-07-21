use ids::UserAvatarFileId;
use sqlx::PgExecutor;

use super::UserAvatarFile;

/// File rows for a batch of ids (resolving many users' display avatars in one round trip).
pub async fn list_user_avatar_files_by_ids<'e, E: PgExecutor<'e>>(
  exec: E,
  ids: &[UserAvatarFileId],
) -> sqlx::Result<Vec<UserAvatarFile>> {
  let ids: Vec<String> = ids.iter().map(|i| i.as_str().to_string()).collect();
  sqlx::query_as!(
    UserAvatarFile,
    r#"SELECT id AS "id: UserAvatarFileId", user_id AS "user_id: ids::UserId",
              original_user_avatar_file_id AS "original_user_avatar_file_id?: UserAvatarFileId",
              avatar_source AS "avatar_source: enums::AvatarFileSource", bucket_hash, content_hash, bucket_suffix, bucket_extension,
              width, height, bytes, created_at
       FROM user_avatar_files
       WHERE id = ANY($1)"#,
    &ids,
  )
  .fetch_all(exec)
  .await
}
