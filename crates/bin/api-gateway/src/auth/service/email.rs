//! Email address change (accounts.md §3).


use db_queries::{
  email_changes, email_verifications, user_avatars, users, User,
};
use enums::AuditEvent;

use crate::auth::email::{canonicalize, canonicalize_clean};
use crate::auth::password::verify_password;
use crate::error::{ApiError, ApiResult};


#[allow(unused_imports)]
use super::*;

/// Change a user's email (accounts.md §2 — the five atomic steps). Requires re-auth (current
/// password). Resets the verified flag, records the change + audit event, invalidates outstanding
/// verifications, and re-runs the avatar pipeline (which re-queues the gravatar probe). Returns the
/// new canonical email.
pub async fn change_email(
  pool: &db::PgPool,
  user: &User,
  new_email: &str,
  current_password: &str,
  ip: Option<&str>,
) -> ApiResult<String> {
  if !new_email.contains('@') {
    return Err(ApiError::BadRequest("that doesn't look like an email address".into()));
  }
  let current_hash = user
    .password_hash
    .as_deref()
    .ok_or_else(|| ApiError::BadRequest("set a password before changing your email".into()))?;
  if !verify_password(current_password, current_hash) {
    return Err(ApiError::Unauthorized("current password is incorrect".into()));
  }

  let new_canon = canonicalize(new_email);
  let new_clean = canonicalize_clean(new_email);
  let new_original = new_email.trim().to_string();

  if new_canon == user.email {
    return Err(ApiError::BadRequest("that is already your email".into()));
  }
  if let Some(existing) = users::find_user_by_email(pool, &new_canon).await? {
    if existing.id != user.id {
      return Err(ApiError::Conflict("that email is already in use".into()));
    }
  }

  let (first, last) = email_letters(&new_canon);
  let gravatar_hash = sha256_hex(&new_canon);

  let mut tx = pool.begin().await?;
  // (1) the email-change history row
  email_changes::create_email_change(
    &mut *tx,
    email_changes::NewEmailChange {
      user_id: &user.id,
      old_email: &user.email,
      old_email_clean: &user.email_clean,
      old_email_original: &user.email_original,
      new_email: &new_canon,
      new_email_clean: &new_clean,
      new_email_original: &new_original,
      change_source: "user",
    },
  )
  .await?;
  // (2) the org-visible audit event
  let data = serde_json::json!({ "old_email": user.email, "new_email": new_canon });
  write_user_event(&mut *tx, &user.id, AuditEvent::EmailChanged, Some(data), ip).await?;
  // (3) re-derive the tiers on `users` (also resets email_verified_at)
  users::update_email(&mut *tx, &user.id, &new_canon, &new_clean, &new_original).await?;
  // (4) invalidate any outstanding verification tokens
  email_verifications::invalidate_email_verifications_for_user(&mut *tx, &user.id).await?;
  // (5) re-run the avatar pipeline → re-queues the gravatar probe (gravatar_checked_at = NULL)
  user_avatars::reset_avatar_for_email_change(&mut *tx, &user.id, &gravatar_hash, first.as_deref(), last.as_deref())
    .await?;
  tx.commit().await?;
  Ok(new_canon)
}
