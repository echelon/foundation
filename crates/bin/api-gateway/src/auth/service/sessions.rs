//! The devices list and session revocation (auth.md §3).


use db_queries::{
  sessions, users, Session, User,
};
use enums::AuditEvent;
use ids::{SessionId, UserId};

use crate::auth::token::JwtKeys;
use crate::error::{ApiError, ApiResult};


#[allow(unused_imports)]
use super::*;

/// List a user's live sessions for the devices screen (auth.md §3).
pub async fn list_sessions(pool: &db::PgPool, user_id: &UserId) -> ApiResult<Vec<Session>> {
  Ok(sessions::list_live_sessions_for_user(pool, user_id).await?)
}

/// Revoke one of the user's sessions by id (auth.md §3). 404s if it isn't theirs. Writes a
/// (user-private) `session_revoked` audit event.
pub async fn revoke_session(
  pool: &db::PgPool,
  user: &User,
  target_session_id: &SessionId,
  ip: Option<&str>,
) -> ApiResult<()> {
  let session = sessions::find_session_by_id(pool, target_session_id)
    .await?
    .filter(|s| s.user_id == user.id)
    .ok_or_else(|| ApiError::NotFound("session not found".into()))?;

  let mut tx = pool.begin().await?;
  sessions::revoke_session(&mut *tx, &session.id).await?;
  write_user_event(&mut *tx, &user.id, AuditEvent::SessionRevoked, None, ip).await?;
  tx.commit().await?;
  Ok(())
}

/// "Log out all other sessions" (auth.md §3): bump `session_version`, revoke every other session
/// row, and re-mint the current cookie at the new `sv`. Returns the new token.
pub async fn revoke_other_sessions(
  pool: &db::PgPool,
  jwt: &JwtKeys,
  user: &User,
  current_session_id: &SessionId,
  ip: Option<&str>,
) -> ApiResult<String> {
  let mut tx = pool.begin().await?;
  let new_sv = users::bump_session_version(&mut *tx, &user.id).await?;
  sessions::revoke_other_sessions(&mut *tx, &user.id, current_session_id).await?;
  let data = serde_json::json!({ "scope": "others" });
  write_user_event(&mut *tx, &user.id, AuditEvent::SessionRevoked, Some(data), ip).await?;
  tx.commit().await?;
  jwt.mint(current_session_id.as_str(), user.id.as_str(), new_sv).map_err(ApiError::Internal)
}
