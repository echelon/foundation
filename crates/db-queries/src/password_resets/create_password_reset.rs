use chrono::{DateTime, Utc};
use ids::{PasswordResetId, UserId};
use sqlx::PgExecutor;

use super::PasswordReset;

pub async fn create_password_reset<'e, E: PgExecutor<'e>>(
  exec: E,
  user_id: &UserId,
  token_hash: &str,
  expires_at: DateTime<Utc>,
) -> sqlx::Result<PasswordReset> {
  let id = PasswordResetId::generate();
  sqlx::query_as!(
    PasswordReset,
    r#"INSERT INTO password_resets (id, user_id, token_hash, expires_at)
       VALUES ($1, $2, $3, $4)
       RETURNING id AS "id: PasswordResetId", user_id AS "user_id: UserId",
                 token_hash, created_at, expires_at, used_at"#,
    id.as_str(),
    user_id.as_str(),
    token_hash,
    expires_at,
  )
  .fetch_one(exec)
  .await
}
