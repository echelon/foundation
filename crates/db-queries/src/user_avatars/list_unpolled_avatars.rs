use sqlx::PgExecutor;

use super::UserAvatar;

/// The gravatar-job's work query: avatars that still need a gravatar existence probe —
/// `gravatar_checked_at IS NULL` and not dismissed. Oldest first, capped at `limit`.
pub async fn list_unpolled_avatars<'e, E: PgExecutor<'e>>(
  exec: E,
  limit: i64,
) -> sqlx::Result<Vec<UserAvatar>> {
  sqlx::query_as!(
    UserAvatar,
    r#"SELECT id AS "id: ids::UserAvatarId", user_id AS "user_id: ids::UserId",
              custom_file_id AS "custom_file_id?: ids::UserAvatarFileId",
              gravatar_file_id AS "gravatar_file_id?: ids::UserAvatarFileId",
              source, gravatar_hash, gravatar_checked_at, is_gravatar_dismissed,
              generated_seed, email_first_letter, email_last_letter,
              person_name_first_letter, person_name_last_letter, created_at, updated_at
       FROM user_avatars
       WHERE gravatar_checked_at IS NULL AND is_gravatar_dismissed = false
       ORDER BY sid ASC
       LIMIT $1"#,
    limit,
  )
  .fetch_all(exec)
  .await
}
