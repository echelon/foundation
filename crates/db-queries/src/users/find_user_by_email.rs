use sqlx::PgExecutor;

use super::User;

/// Resolve a user by canonicalized email (the unique login key).
pub async fn find_user_by_email<'e, E: PgExecutor<'e>>(exec: E, email: &str) -> sqlx::Result<Option<User>> {
  sqlx::query_as!(
    User,
    r#"SELECT id AS "id: ids::UserId", email, email_clean, email_original, email_verified_at,
              password_hash, session_version, display_name, is_staff, created_at
       FROM users
       WHERE email = $1 AND deleted_at IS NULL"#,
    email,
  )
  .fetch_optional(exec)
  .await
}
