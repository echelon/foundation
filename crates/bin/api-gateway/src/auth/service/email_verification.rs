//! Email verification: request + confirm (accounts.md §3).

use chrono::Utc;

use db_queries::{
  email_verifications, users, User,
};
use enums::AuditEvent;

use crate::error::{ApiError, ApiResult};


#[allow(unused_imports)]
use super::*;

/// A minted email verification ready to send: the plaintext token for the link (only its sha256
/// is stored) and the address it verifies.
pub struct PendingVerification {
  pub to_email: String,
  pub token: String,
}

/// Start email verification (accounts.md §2): invalidate outstanding tokens, mint a fresh one
/// (24h, single-use), snapshotting the current email — a token can never verify an address it
/// wasn't minted for. Errors if the email is already verified.
pub async fn request_email_verification(pool: &db::PgPool, user: &User) -> ApiResult<PendingVerification> {
  if user.email_verified_at.is_some() {
    return Err(ApiError::BadRequest("this email is already verified".into()));
  }
  let token = gen_token();
  let token_hash = sha256_hex(&token);
  let expires_at = Utc::now() + chrono::Duration::hours(24);

  let mut tx = pool.begin().await?;
  email_verifications::invalidate_email_verifications_for_user(&mut *tx, &user.id).await?;
  email_verifications::create_email_verification(
    &mut *tx,
    email_verifications::NewEmailVerification {
      user_id: &user.id,
      email: &user.email,
      token_hash: &token_hash,
      expires_at,
    },
  )
  .await?;
  tx.commit().await?;
  Ok(PendingVerification { to_email: user.email.clone(), token })
}

/// Consume a verification token (accounts.md §2): single-use, unexpired, and the snapshotted
/// email must STILL be the user's email. Sets `email_verified_at` and writes the ORG-VISIBLE
/// `email_verified` audit event (audit_logs.md §3) atomically.
pub async fn confirm_email_verification(pool: &db::PgPool, token: &str, ip: Option<&str>) -> ApiResult<()> {
  const BAD: &str = "this verification link is invalid or has expired";
  let token_hash = sha256_hex(token);
  let verification = email_verifications::find_email_verification_by_token_hash(pool, &token_hash)
    .await?
    .ok_or_else(|| ApiError::BadRequest(BAD.into()))?;
  let user = users::find_user_by_id(pool, &verification.user_id)
    .await?
    .ok_or_else(|| ApiError::BadRequest(BAD.into()))?;

  // The snapshot guard: a token minted for an old address can't verify a new one.
  if verification.email != user.email {
    return Err(ApiError::BadRequest(BAD.into()));
  }
  if user.email_verified_at.is_some() {
    return Ok(()); // idempotent — clicking the link twice isn't an error
  }

  let mut tx = pool.begin().await?;
  // Atomically claim the token — false means already used/expired.
  if !email_verifications::mark_email_verification_validated(&mut *tx, &verification.id).await? {
    return Err(ApiError::BadRequest(BAD.into()));
  }
  users::mark_email_verified(&mut *tx, &user.id).await?;
  let data = serde_json::json!({ "email": user.email });
  write_user_event(&mut *tx, &user.id, AuditEvent::EmailVerified, Some(data), ip).await?;
  tx.commit().await?;
  Ok(())
}
