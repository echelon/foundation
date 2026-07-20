use chrono::{DateTime, Utc};
use ids::UserId;
use sqlx::PgExecutor;

use super::User;

/// Fields for creating a user. `email` is the canonicalized (trim + lowercase) login key;
/// `email_clean` is the aggressive canonicalization; `email_original` is verbatim.
#[derive(Debug, Clone)]
pub struct NewUser {
  pub email: String,
  pub email_clean: String,
  pub email_original: String,
  pub password_hash: Option<String>,
  pub display_name: Option<String>,
  /// Set for SSO signups (provider asserts the email); null for email/password at launch.
  pub email_verified_at: Option<DateTime<Utc>>,
}

pub async fn create_user<'e, E: PgExecutor<'e>>(exec: E, new: NewUser) -> sqlx::Result<User> {
  let id = UserId::generate();
  sqlx::query_as!(
    User,
    r#"INSERT INTO users (id, email, email_clean, email_original, password_hash, display_name, email_verified_at)
       VALUES ($1, $2, $3, $4, $5, $6, $7)
       RETURNING id AS "id: UserId", email, email_clean, email_original, email_verified_at,
                 password_hash, session_version, display_name, is_staff, created_at"#,
    id.as_str(),
    new.email,
    new.email_clean,
    new.email_original,
    new.password_hash,
    new.display_name,
    new.email_verified_at,
  )
  .fetch_one(exec)
  .await
}
