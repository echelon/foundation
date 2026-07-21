use ids::{UserAvatarId, UserId};
use sqlx::PgExecutor;

use super::UserAvatar;

/// Fields for the initial avatar row created in the signup transaction. `gravatar_checked_at`
/// is left NULL (the "needs poll" signal); `preferred_source` takes the DB default `gravatar`,
/// so a mirror displays automatically once the job finds one — until then the letter monogram
/// renders (the preference isn't satisfiable yet).
#[derive(Debug, Clone)]
pub struct NewUserAvatar<'a> {
  pub user_id: &'a UserId,
  pub gravatar_hash: &'a str,
  pub generated_seed: &'a str,
  pub email_first_letter: Option<&'a str>,
  pub email_last_letter: Option<&'a str>,
}

pub async fn create_user_avatar<'e, E: PgExecutor<'e>>(
  exec: E,
  new: NewUserAvatar<'_>,
) -> sqlx::Result<UserAvatar> {
  let id = UserAvatarId::generate();
  sqlx::query_as!(
    UserAvatar,
    r#"INSERT INTO user_avatars
         (id, user_id, gravatar_hash, generated_seed, email_first_letter, email_last_letter)
       VALUES ($1, $2, $3, $4, $5, $6)
       RETURNING id AS "id: UserAvatarId", user_id AS "user_id: UserId",
                 custom_file_id AS "custom_file_id?: ids::UserAvatarFileId",
                 gravatar_file_id AS "gravatar_file_id?: ids::UserAvatarFileId",
                 preferred_source AS "preferred_source: enums::AvatarSource", gravatar_hash, gravatar_checked_at,
                 generated_seed, email_first_letter, email_last_letter,
                 person_name_first_letter, person_name_last_letter, created_at, updated_at,
              monogram_gradient AS "monogram_gradient?: enums::MonogramGradient""#,
    id.as_str(),
    new.user_id.as_str(),
    new.gravatar_hash,
    new.generated_seed,
    new.email_first_letter,
    new.email_last_letter,
  )
  .fetch_one(exec)
  .await
}
