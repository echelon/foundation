//! Forgot-password reset flow: token mint + consume (auth.md §3).

use chrono::Utc;

use db_queries::{
  password_resets, users,
};
use enums::AuditEvent;
use ids::UserId;

use crate::auth::email::canonicalize;
use crate::auth::password::hash_password;
use crate::error::{ApiError, ApiResult};


#[allow(unused_imports)]
use super::*;

/// A minted password reset ready to email. Returned by [`begin_password_reset`] only when the
/// address actually maps to a user — the handler sends the email and, either way, returns an
/// identical response so existence is never revealed.
pub struct PendingReset {
  pub user_id: UserId,
  pub to_email: String,
  /// Plaintext token for the emailed link (DB stores only its sha256).
  pub token: String,
}

/// Start a password reset (accounts.md §2, auth.md §3). Creates a single-use, ~1h token if the
/// email maps to a user; returns `None` otherwise (the caller responds the same either way).
pub async fn begin_password_reset(pool: &db::PgPool, email: &str) -> ApiResult<Option<PendingReset>> {
  let email_canon = canonicalize(email);
  let Some(user) = users::find_user_by_email(pool, &email_canon).await? else {
    return Ok(None);
  };
  let token = gen_token();
  let token_hash = sha256_hex(&token);
  let expires_at = Utc::now() + chrono::Duration::hours(1);
  password_resets::create_password_reset(pool, &user.id, &token_hash, expires_at).await?;
  Ok(Some(PendingReset { user_id: user.id, to_email: user.email, token }))
}

/// Consume a password-reset token (accounts.md §2). Single-use + unexpired; sets the new password,
/// bumps `password_version` (record), writes `password_reset` audit, and — only if the user asked —
/// bumps `session_version` to end all existing sessions.
pub async fn consume_password_reset(
  pool: &db::PgPool,
  token: &str,
  new_password: &str,
  logout_others: bool,
  ip: Option<&str>,
) -> ApiResult<()> {
  if new_password.len() < MIN_PASSWORD_LEN {
    return Err(ApiError::BadRequest(format!("password must be at least {MIN_PASSWORD_LEN} characters")));
  }
  let token_hash = sha256_hex(token);
  let reset = password_resets::find_password_reset_by_token_hash(pool, &token_hash)
    .await?
    .ok_or_else(|| ApiError::BadRequest("this reset link is invalid or has expired".into()))?;
  let user = users::find_user_by_id(pool, &reset.user_id)
    .await?
    .ok_or_else(|| ApiError::BadRequest("this reset link is invalid or has expired".into()))?;

  let new_hash = hash_password(new_password).map_err(ApiError::Internal)?;
  let mut tx = pool.begin().await?;
  // Atomically claim the token — 0 rows means already used/expired.
  if !password_resets::mark_password_reset_used(&mut *tx, &reset.id).await? {
    return Err(ApiError::BadRequest("this reset link is invalid or has expired".into()));
  }
  users::set_password_hash(&mut *tx, &user.id, &new_hash).await?;
  write_user_event(&mut *tx, &user.id, AuditEvent::PasswordReset, None, ip).await?;
  if logout_others {
    // No "current" session in an unauthenticated reset — bumping sv ends every existing session.
    users::bump_session_version(&mut *tx, &user.id).await?;
  }
  tx.commit().await?;
  Ok(())
}
