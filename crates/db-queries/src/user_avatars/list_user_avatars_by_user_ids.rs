use ids::UserId;
use sqlx::PgExecutor;

use super::UserAvatar;

/// Avatar rows for a batch of users (roster / audit-actor rendering). Users with no row are
/// simply absent — callers fall back to the letter monogram.
pub async fn list_user_avatars_by_user_ids<'e, E: PgExecutor<'e>>(
  exec: E,
  user_ids: &[UserId],
) -> sqlx::Result<Vec<UserAvatar>> {
  let ids: Vec<String> = user_ids.iter().map(|u| u.as_str().to_string()).collect();
  sqlx::query_as!(
    UserAvatar,
    r#"SELECT id AS "id: ids::UserAvatarId", user_id AS "user_id: UserId",
              custom_file_id AS "custom_file_id?: ids::UserAvatarFileId",
              gravatar_file_id AS "gravatar_file_id?: ids::UserAvatarFileId",
              preferred_source AS "preferred_source: enums::AvatarSource", gravatar_hash, gravatar_checked_at,
              generated_seed, email_first_letter, email_last_letter,
              person_name_first_letter, person_name_last_letter, created_at, updated_at
       FROM user_avatars
       WHERE user_id = ANY($1)"#,
    &ids,
  )
  .fetch_all(exec)
  .await
}
