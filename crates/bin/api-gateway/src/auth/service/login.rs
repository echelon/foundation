//! Email/password login and logout (auth.md §2).


use db_queries::{
  sessions, users,
};
use enums::SessionCreationType;
use ids::SessionId;

use crate::auth::email::canonicalize;
use crate::auth::password::verify_password;
use crate::auth::token::JwtKeys;
use crate::error::{ApiError, ApiResult};


#[allow(unused_imports)]
use super::*;

/// Email + password login (auth.md §2). Generic failure — never reveals whether the email
/// exists.
pub async fn login(
  pool: &db::PgPool,
  jwt: &JwtKeys,
  email: &str,
  password: &str,
  meta: &ReqMeta,
) -> ApiResult<AuthOutcome> {
  let email_canon = canonicalize(email);
  let user = users::find_user_by_email(pool, &email_canon).await?;

  let user = match user {
    Some(u) => u,
    None => return Err(ApiError::Unauthorized("invalid email or password".into())),
  };
  let ok = user
    .password_hash
    .as_deref()
    .map(|h| verify_password(password, h))
    .unwrap_or(false);
  if !ok {
    return Err(ApiError::Unauthorized("invalid email or password".into()));
  }
  start_session(pool, jwt, user, None, meta, SessionCreationType::Login).await
}

/// Log out — revoke the current session (auth.md §3, path 2).
pub async fn logout(pool: &db::PgPool, session_id: &SessionId) -> ApiResult<()> {
  sessions::revoke_session(pool, session_id).await?;
  Ok(())
}
