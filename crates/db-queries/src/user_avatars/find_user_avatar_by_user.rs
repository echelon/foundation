use ids::UserId;
use sqlx::PgExecutor;

use super::UserAvatar;

pub async fn find_user_avatar_by_user<'e, E: PgExecutor<'e>>(
  exec: E,
  user_id: &UserId,
) -> sqlx::Result<Option<UserAvatar>> {
  sqlx::query_as!(
    UserAvatar,
    r#"SELECT id AS "id: ids::UserAvatarId", user_id AS "user_id: UserId",
              custom_file_id AS "custom_file_id?: ids::UserAvatarFileId",
              gravatar_file_id AS "gravatar_file_id?: ids::UserAvatarFileId",
              source, gravatar_hash, gravatar_checked_at, is_gravatar_dismissed,
              generated_seed, email_first_letter, email_last_letter,
              person_name_first_letter, person_name_last_letter, created_at, updated_at
       FROM user_avatars
       WHERE user_id = $1"#,
    user_id.as_str(),
  )
  .fetch_optional(exec)
  .await
}
