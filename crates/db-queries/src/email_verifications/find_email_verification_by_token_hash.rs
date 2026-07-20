use sqlx::PgExecutor;

use super::EmailVerification;

/// Resolve a verification from the emailed token's sha256. The caller must additionally check the
/// row is unexpired, unvalidated, and that `row.email == users.email` (accounts.md §2).
pub async fn find_email_verification_by_token_hash<'e, E: PgExecutor<'e>>(
  exec: E,
  token_hash: &str,
) -> sqlx::Result<Option<EmailVerification>> {
  sqlx::query_as!(
    EmailVerification,
    r#"SELECT id AS "id: ids::EmailVerificationId", user_id AS "user_id: ids::UserId",
              email, token_hash, created_at, expires_at, validated_at
       FROM email_verifications
       WHERE token_hash = $1"#,
    token_hash,
  )
  .fetch_optional(exec)
  .await
}
