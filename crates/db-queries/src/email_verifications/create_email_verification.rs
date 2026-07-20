use chrono::{DateTime, Utc};
use ids::{EmailVerificationId, UserId};
use sqlx::PgExecutor;

use super::EmailVerification;

#[derive(Debug, Clone)]
pub struct NewEmailVerification<'a> {
  pub user_id: &'a UserId,
  pub email: &'a str,
  pub token_hash: &'a str,
  pub expires_at: DateTime<Utc>,
}

pub async fn create_email_verification<'e, E: PgExecutor<'e>>(
  exec: E,
  new: NewEmailVerification<'_>,
) -> sqlx::Result<EmailVerification> {
  let id = EmailVerificationId::generate();
  sqlx::query_as!(
    EmailVerification,
    r#"INSERT INTO email_verifications (id, user_id, email, token_hash, expires_at)
       VALUES ($1, $2, $3, $4, $5)
       RETURNING id AS "id: EmailVerificationId", user_id AS "user_id: UserId",
                 email, token_hash, created_at, expires_at, validated_at"#,
    id.as_str(),
    new.user_id.as_str(),
    new.email,
    new.token_hash,
    new.expires_at,
  )
  .fetch_one(exec)
  .await
}
