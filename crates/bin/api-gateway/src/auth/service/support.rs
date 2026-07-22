//! Shared internals: session minting, new-user provisioning, audit writes, token/code generation.

use chrono::Utc;
use sha2::{Digest, Sha256};

use db_queries::{
  account_audit_logs, sessions, user_audit_logs, user_avatars, Account, Session, User,
};
use enums::{AuditEvent, SessionCreationType};
use ids::UserId;

use crate::auth::token::{JwtKeys, SESSION_TTL};
use crate::error::{ApiError, ApiResult};


#[allow(unused_imports)]
use super::*;

pub(crate) const MIN_PASSWORD_LEN: usize = 8;

/// First letter of the first word and first letter of the last word, uppercased — the
/// person-name monogram (accounts.md §2).
pub(crate) fn person_letters(name: Option<&str>) -> (Option<String>, Option<String>) {
  let Some(name) = name else { return (None, None) };
  let words: Vec<&str> = name.split_whitespace().collect();
  let first = words.first().and_then(|w| w.chars().next()).map(|c| c.to_uppercase().to_string());
  let last = if words.len() > 1 {
    words.last().and_then(|w| w.chars().next()).map(|c| c.to_uppercase().to_string())
  } else {
    None
  };
  (first, last)
}

/// Create a session row for an existing user and mint its token.
pub(crate) async fn start_session(
  pool: &db::PgPool,
  jwt: &JwtKeys,
  user: User,
  impersonator_user_id: Option<&UserId>,
  meta: &ReqMeta,
  created_via: SessionCreationType,
) -> ApiResult<AuthOutcome> {
  let session = sessions::create_session(
    pool,
    sessions::NewSession {
      user_id: &user.id,
      expires_at: Utc::now() + SESSION_TTL,
      ip: meta.ip.as_deref(),
      user_agent: meta.user_agent.as_deref(),
      device: None,
      impersonator_user_id,
      created_via,
    },
  )
  .await?;
  let token = jwt
    .mint(session.id.as_str(), user.id.as_str(), user.session_version)
    .map_err(ApiError::Internal)?;
  Ok(AuthOutcome { user, session, token })
}

/// Provision the non-essential-but-standard bits of a new user, inside the signup transaction:
/// the avatar row (with a computed gravatar hash + letters, left UNPOLLED so the gravatar-job
/// picks it up) and the `account_created` audit events (U + A — accounts.md, audit_logs.md §3).
pub(crate) async fn provision_new_user(
  tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
  user: &User,
  account: &Account,
  meta: &ReqMeta,
) -> ApiResult<()> {
  create_default_avatar(tx, user).await?;

  let ip = meta.ip.as_deref();
  write_user_event(&mut **tx, &user.id, AuditEvent::AccountCreated, None, ip).await?;
  account_audit_logs::create_account_audit_log(
    &mut **tx,
    account_audit_logs::NewAccountAuditLog {
      account_id: &account.id,
      actor_user_id: Some(&user.id),
      user_id: Some(&user.id),
      event: AuditEvent::AccountCreated,
      target_type: None,
      target_id: None,
      data: None,
      ip,
    },
  )
  .await?;
  Ok(())
}

/// The avatar row every new user gets (computed gravatar hash + monogram letters, left
/// UNPOLLED so the gravatar-job picks it up) — shared by normal and invited signup.
pub(crate) async fn create_default_avatar(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, user: &User) -> ApiResult<()> {
  let (first, last) = email_letters(&user.email);
  // preferred_source defaults to `gravatar` — the monogram shows until a mirror lands.
  user_avatars::create_user_avatar(
    &mut **tx,
    user_avatars::NewUserAvatar {
      user_id: &user.id,
      gravatar_hash: &sha256_hex(&user.email),
      generated_seed: user.id.as_str(),
      email_first_letter: first.as_deref(),
      email_last_letter: last.as_deref(),
    },
  )
  .await?;
  Ok(())
}

/// Write one `user_audit_logs` row (the actor is the subject themselves); org visibility is
/// derived from the event (`AuditEvent::is_org_visible`, audit_logs.md §3).
pub(crate) async fn write_user_event<'e, E: sqlx::PgExecutor<'e>>(
  exec: E,
  user_id: &UserId,
  event: AuditEvent,
  data: Option<serde_json::Value>,
  ip: Option<&str>,
) -> ApiResult<()> {
  user_audit_logs::create_user_audit_log(
    exec,
    user_audit_logs::NewUserAuditLog {
      user_id,
      actor_user_id: Some(user_id),
      event,
      target_type: None,
      target_id: None,
      data,
      ip,
    },
  )
  .await?;
  Ok(())
}

/// First and last letter of the email's local part (before `@`), uppercased, for the monogram
/// fallback avatar. Returns `(None, None)` for a degenerate address.
pub(crate) fn email_letters(email: &str) -> (Option<String>, Option<String>) {
  let local = email.split('@').next().unwrap_or("");
  let chars: Vec<char> = local.chars().filter(|c| c.is_alphanumeric()).collect();
  let first = chars.first().map(|c| c.to_uppercase().to_string());
  let last = chars.last().map(|c| c.to_uppercase().to_string());
  (first, last)
}

/// Create a session inside an open transaction (signup flows).
pub(crate) async fn new_session(
  tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
  user_id: &UserId,
  meta: &ReqMeta,
  created_via: SessionCreationType,
) -> ApiResult<Session> {
  let session = sessions::create_session(
    &mut **tx,
    sessions::NewSession {
      user_id,
      expires_at: Utc::now() + SESSION_TTL,
      ip: meta.ip.as_deref(),
      user_agent: meta.user_agent.as_deref(),
      device: None,
      impersonator_user_id: None,
      created_via,
    },
  )
  .await?;
  Ok(session)
}

pub(crate) fn sha256_hex(input: &str) -> String {
  let mut hasher = Sha256::new();
  hasher.update(input.as_bytes());
  hex::encode(hasher.finalize())
}

/// A 32-hex-char magic-link token (128 bits).
pub(crate) fn gen_token() -> String {
  use rand::Rng;
  let bytes: [u8; 16] = rand::thread_rng().gen();
  hex::encode(bytes)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn code_shape_is_xxxx_dash_xxxx() {
    let code = gen_code();
    assert_eq!(code.len(), 9);
    assert_eq!(&code[4..5], "-");
  }

  #[test]
  fn token_is_32_hex_chars() {
    let t = gen_token();
    assert_eq!(t.len(), 32);
    assert!(t.chars().all(|c| c.is_ascii_hexdigit()));
  }

  #[test]
  fn sha256_hex_is_stable_and_64_chars() {
    let a = sha256_hex("hello");
    assert_eq!(a.len(), 64);
    assert_eq!(a, sha256_hex("hello"));
    assert_ne!(a, sha256_hex("world"));
  }
}
