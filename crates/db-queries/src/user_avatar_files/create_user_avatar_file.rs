use enums::AvatarFileSource;
use ids::{UserAvatarFileId, UserId};
use sqlx::PgExecutor;

use super::UserAvatarFile;

#[derive(Debug, Clone)]
pub struct NewUserAvatarFile<'a> {
  pub user_id: &'a UserId,
  /// Set when this row is a baked crop of another row.
  pub original_user_avatar_file_id: Option<&'a UserAvatarFileId>,
  pub avatar_source: AvatarFileSource,
  /// 32-char Crockford entropy (`bucket_paths::mint_bucket_hash`).
  pub bucket_hash: &'a str,
  /// sha256 of the content (integrity — distinct job from the entropy hash).
  pub content_hash: &'a str,
  /// e.g. `_512` on baked display crops.
  pub bucket_suffix: Option<&'a str>,
  /// Includes the leading period, e.g. `.png`.
  pub bucket_extension: &'a str,
  pub width: Option<i32>,
  pub height: Option<i32>,
  pub bytes: i64,
}

pub async fn create_user_avatar_file<'e, E: PgExecutor<'e>>(
  exec: E,
  new: NewUserAvatarFile<'_>,
) -> sqlx::Result<UserAvatarFile> {
  let id = UserAvatarFileId::generate();
  let original = new.original_user_avatar_file_id.map(|f| f.as_str());
  sqlx::query_as!(
    UserAvatarFile,
    r#"INSERT INTO user_avatar_files
         (id, user_id, original_user_avatar_file_id, avatar_source, bucket_hash, content_hash,
          bucket_suffix, bucket_extension, width, height, bytes)
       VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
       RETURNING id AS "id: UserAvatarFileId", user_id AS "user_id: UserId",
                 original_user_avatar_file_id AS "original_user_avatar_file_id?: UserAvatarFileId",
                 avatar_source AS "avatar_source: AvatarFileSource", bucket_hash, content_hash, bucket_suffix, bucket_extension,
                 width, height, bytes, created_at"#,
    id.as_str(),
    new.user_id.as_str(),
    original,
    new.avatar_source.as_str(),
    new.bucket_hash,
    new.content_hash,
    new.bucket_suffix,
    new.bucket_extension,
    new.width,
    new.height,
    new.bytes,
  )
  .fetch_one(exec)
  .await
}
