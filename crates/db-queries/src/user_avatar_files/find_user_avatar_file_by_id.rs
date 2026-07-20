use ids::UserAvatarFileId;
use sqlx::PgExecutor;

use super::UserAvatarFile;

pub async fn find_user_avatar_file_by_id<'e, E: PgExecutor<'e>>(
  exec: E,
  id: &UserAvatarFileId,
) -> sqlx::Result<Option<UserAvatarFile>> {
  sqlx::query_as!(
    UserAvatarFile,
    r#"SELECT id AS "id: UserAvatarFileId", user_id AS "user_id: ids::UserId",
              original_user_avatar_file_id AS "original_user_avatar_file_id?: UserAvatarFileId",
              avatar_source AS "avatar_source: enums::AvatarFileSource", bucket_hash, content_hash, bucket_suffix, bucket_extension,
              width, height, bytes, created_at
       FROM user_avatar_files
       WHERE id = $1"#,
    id.as_str(),
  )
  .fetch_optional(exec)
  .await
}
