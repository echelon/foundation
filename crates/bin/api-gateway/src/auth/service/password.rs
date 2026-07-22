//! Password change (with session remint) — auth.md §3.


use db_queries::{
  sessions, users, User,
};
use enums::AuditEvent;
use ids::SessionId;

use crate::auth::password::{hash_password, verify_password};
use crate::auth::token::JwtKeys;
use crate::error::{ApiError, ApiResult};


#[allow(unused_imports)]
use super::*;

/// The result of a flow that may re-mint the current session cookie. When the user opts to "log
/// out my other devices", we bump `session_version` — which would also invalidate the CURRENT
/// session's JWT — so we hand back a freshly-minted token to re-set the cookie at the new `sv`
/// (auth.md §3).
pub struct SessionReminted {
  /// A new token to set as the cookie, present only when other sessions were purged.
  pub new_token: Option<String>,
}

/// Change password for an authenticated user (accounts.md §6, auth.md §3). Requires fresh re-auth
/// (the current password). Always bumps `password_version` (a record). Other sessions are revoked
/// ONLY if the user opts in — and that opt-in is what bumps `session_version`.
/// Everything `change_password` needs beyond identities (named fields — the two-password
/// `&str` run is exactly the transposition hazard the args-struct rule exists for).
pub struct ChangePasswordArgs<'a> {
  /// Required when a password exists; MUST be absent/ignored for SSO-only users setting
  /// their first one.
  pub current_password: Option<&'a str>,
  pub new_password: &'a str,
  /// Also revoke every OTHER session (bumps the session-version clock; this cookie re-mints).
  pub logout_others: bool,
  pub current_session_id: &'a SessionId,
  pub ip: Option<&'a str>,
}

pub async fn change_password(
  pool: &db::PgPool,
  jwt: &JwtKeys,
  user: &User,
  args: ChangePasswordArgs<'_>,
) -> ApiResult<SessionReminted> {
  let ChangePasswordArgs { current_password, new_password, logout_others, current_session_id, ip } = args;
  if new_password.len() < MIN_PASSWORD_LEN {
    return Err(ApiError::BadRequest(format!("password must be at least {MIN_PASSWORD_LEN} characters")));
  }
  // Fresh re-auth: confirm the current password — except for SSO-only users, who have none
  // to confirm. They SET their first password from a live (online-checked) session.
  // (accounts.md §5 sketches a fresh SSO re-auth here; relaxed per bt, 2026-07-21.)
  let first_password = match user.password_hash.as_deref() {
    Some(current_hash) => {
      let supplied =
        current_password.ok_or_else(|| ApiError::BadRequest("current password is required".into()))?;
      if !verify_password(supplied, current_hash) {
        return Err(ApiError::Unauthorized("current password is incorrect".into()));
      }
      false
    }
    None => true,
  };

  let new_hash = hash_password(new_password).map_err(ApiError::Internal)?;
  let mut tx = pool.begin().await?;
  users::set_password_hash(&mut *tx, &user.id, &new_hash).await?;
  let data = first_password.then(|| serde_json::json!({ "first_password": true }));
  write_user_event(&mut *tx, &user.id, AuditEvent::PasswordChanged, data, ip).await?;

  let new_token = if logout_others {
    let new_sv = users::bump_session_version(&mut *tx, &user.id).await?;
    sessions::revoke_other_sessions(&mut *tx, &user.id, current_session_id).await?;
    Some(jwt.mint(current_session_id.as_str(), user.id.as_str(), new_sv).map_err(ApiError::Internal)?)
  } else {
    None
  };
  tx.commit().await?;
  Ok(SessionReminted { new_token })
}
