use ids::UserId;
use sqlx::PgExecutor;

use super::User;

pub async fn find_user_by_id<'e, E: PgExecutor<'e>>(exec: E, id: &UserId) -> sqlx::Result<Option<User>> {
  sqlx::query_as!(
    User,
    r#"SELECT id AS "id: UserId", email, email_clean, email_original, email_verified_at,
              password_hash, session_version, display_name, is_staff, created_at
       FROM users
       WHERE id = $1 AND deleted_at IS NULL"#,
    id.as_str(),
  )
  .fetch_optional(exec)
  .await
}
