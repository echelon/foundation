//! Auth flow logic (accounts.md §3–§5, auth.md §2). Handlers are thin wrappers over these.
//! The signup/SSO-signup flows run in ONE transaction (user + account + owner membership +
//! session commit atomically — accounts.md §3).

use chrono::Utc;
use sha2::{Digest, Sha256};

use db_queries::{accounts, invitations, members, sessions, sso, users, Invitation, Session, User};
use ids::{AccountId, InvitationId, SessionId, UserId};

use crate::auth::email::{canonicalize, canonicalize_clean};
use crate::auth::password::{hash_password, verify_password};
use crate::auth::token::{JwtKeys, SESSION_TTL};
use crate::error::{ApiError, ApiResult};

/// Request metadata captured for the session row / audit.
#[derive(Debug, Clone, Default)]
pub struct ReqMeta {
  pub ip: Option<String>,
  pub user_agent: Option<String>,
}

/// The result of any flow that ends logged-in: the user, their new session, and the JWT to
/// set as a cookie.
pub struct AuthOutcome {
  pub user: User,
  pub session: Session,
  pub token: String,
}

const MIN_PASSWORD_LEN: usize = 8;

/// Email + password signup (accounts.md §3A), including the signup-as-login rule.
pub async fn signup(
  pool: &db::PgPool,
  jwt: &JwtKeys,
  email: &str,
  password: &str,
  display_name: Option<&str>,
  meta: &ReqMeta,
) -> ApiResult<AuthOutcome> {
  if password.len() < MIN_PASSWORD_LEN {
    return Err(ApiError::BadRequest(format!("password must be at least {MIN_PASSWORD_LEN} characters")));
  }
  let email_canon = canonicalize(email);

  // Existing user? signup-as-login when the password is correct; otherwise route to login.
  if let Some(user) = users::find_user_by_email(pool, &email_canon).await? {
    match &user.password_hash {
      Some(hash) if verify_password(password, hash) => {
        return start_session(pool, jwt, user, None, meta).await;
      }
      _ => {
        return Err(ApiError::Conflict(
          "an account with this email already exists — log in instead".into(),
        ));
      }
    }
  }

  // Genuinely new — create everything atomically.
  let hash = hash_password(password).map_err(ApiError::Internal)?;
  let mut tx = pool.begin().await?;

  let user = users::create_user(
    &mut *tx,
    users::NewUser {
      email: email_canon,
      email_clean: canonicalize_clean(email),
      email_original: email.trim().to_string(),
      password_hash: Some(hash),
      display_name: display_name.map(str::to_string),
      email_verified_at: None,
    },
  )
  .await?;
  let account = accounts::create_account(&mut *tx, None).await?;
  members::create_membership(&mut *tx, &account.id, &user.id, "owner").await?;
  let session = new_session(&mut tx, &user.id, meta).await?;

  tx.commit().await?;

  let token = jwt
    .mint(session.id.as_str(), user.id.as_str(), user.session_version)
    .map_err(ApiError::Internal)?;
  Ok(AuthOutcome { user, session, token })
}

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
  start_session(pool, jwt, user, None, meta).await
}

/// SSO login/signup (accounts.md §3B/C, §5). Resolves on `(provider, provider_user_id)` —
/// NEVER on email. A matching email with no existing connection is a hard stop (no silent
/// merge — prevents takeover). `provider_user_id` is the stable `sub`/numeric id.
pub async fn oauth_login(
  pool: &db::PgPool,
  jwt: &JwtKeys,
  provider: &str,
  provider_user_id: &str,
  email: &str,
  display_name: Option<&str>,
  meta: &ReqMeta,
) -> ApiResult<AuthOutcome> {
  if provider != "google" && provider != "github" {
    return Err(ApiError::BadRequest(format!("unsupported provider: {provider}")));
  }

  // 1. Existing connection → log in.
  if let Some(conn) = sso::find_sso_by_provider_sub(pool, provider, provider_user_id).await? {
    sso::touch_sso_last_used(pool, &conn.id).await?;
    let user = users::find_user_by_id(pool, &conn.user_id)
      .await?
      .ok_or_else(|| ApiError::Internal("sso connection references missing user".into()))?;
    return start_session(pool, jwt, user, None, meta).await;
  }

  // 2. No connection but the email already has an account → do NOT merge.
  let email_canon = canonicalize(email);
  if users::find_user_by_email(pool, &email_canon).await?.is_some() {
    return Err(ApiError::Conflict(
      "an account with this email exists — log in to link this provider".into(),
    ));
  }

  // 3. Genuinely new → create user (verified, no password) + connection + account atomically.
  let mut tx = pool.begin().await?;
  let user = users::create_user(
    &mut *tx,
    users::NewUser {
      email: email_canon,
      email_clean: canonicalize_clean(email),
      email_original: email.trim().to_string(),
      password_hash: None,
      display_name: display_name.map(str::to_string),
      email_verified_at: Some(Utc::now()),
    },
  )
  .await?;
  sso::create_sso_connection(
    &mut *tx,
    sso::NewSsoConnection {
      user_id: &user.id,
      provider,
      provider_user_id,
      provider_email: &canonicalize(email),
      provider_user_display_name: display_name,
    },
  )
  .await?;
  let account = accounts::create_account(&mut *tx, None).await?;
  members::create_membership(&mut *tx, &account.id, &user.id, "owner").await?;
  let session = new_session(&mut tx, &user.id, meta).await?;
  tx.commit().await?;

  let token = jwt
    .mint(session.id.as_str(), user.id.as_str(), user.session_version)
    .map_err(ApiError::Internal)?;
  Ok(AuthOutcome { user, session, token })
}

/// Log out — revoke the current session (auth.md §3, path 2).
pub async fn logout(pool: &db::PgPool, session_id: &SessionId) -> ApiResult<()> {
  sessions::revoke_session(pool, session_id).await?;
  Ok(())
}

/// A created invitation plus the plaintext token/code to hand to the invitee (the token is
/// only ever returned here — the DB stores its sha256).
pub struct CreatedInvitation {
  pub invitation: Invitation,
  /// The magic-link token (goes in the emailed link, never persisted in plaintext).
  pub token: String,
  /// The short human code (e.g. `7F3K-QT9M`).
  pub code: String,
}

/// Create an invitation to an account (accounts.md §4). Owner/admin only (authorized by the
/// handler). Invites grant `admin`/`member`, never `owner`; re-inviting revokes the prior one.
pub async fn create_invitation(
  pool: &db::PgPool,
  account_id: &AccountId,
  inviter_user_id: &UserId,
  invitee_email: &str,
  role: &str,
) -> ApiResult<CreatedInvitation> {
  if role != "admin" && role != "member" {
    return Err(ApiError::BadRequest("invite role must be 'admin' or 'member'".into()));
  }
  let email_canon = canonicalize(invitee_email);

  // Controller rule: reject inviting an email that is already a member of the account.
  if let Some(existing) = users::find_user_by_email(pool, &email_canon).await? {
    if members::find_membership(pool, account_id, &existing.id).await?.is_some() {
      return Err(ApiError::Conflict("that email is already a member of this account".into()));
    }
  }

  let mut tx = pool.begin().await?;
  invitations::revoke_outstanding_invitations(&mut *tx, account_id, &email_canon).await?;

  let code = gen_code();
  let token = gen_token();
  let token_hash = sha256_hex(&token);
  let expires_at = Utc::now() + chrono::Duration::days(7);

  let invitation = invitations::create_invitation(
    &mut *tx,
    invitations::NewInvitation {
      account_id,
      invited_by_user_id: inviter_user_id,
      email: &email_canon,
      role,
      code: &code,
      token_hash: &token_hash,
      expires_at,
    },
  )
  .await?;
  tx.commit().await?;

  Ok(CreatedInvitation { invitation, token, code })
}

/// Resolve an invitation from its code or magic-link token, for the join page.
pub async fn resolve_invitation(pool: &db::PgPool, code_or_token: &str) -> ApiResult<Invitation> {
  // Try the short code first, then the token (hashed).
  let by_code = invitations::find_invitation_by_code(pool, code_or_token).await?;
  let inv = match by_code {
    Some(i) => i,
    None => invitations::find_invitation_by_token_hash(pool, &sha256_hex(code_or_token))
      .await?
      .ok_or_else(|| ApiError::NotFound("invitation not found".into()))?,
  };
  ensure_invitation_open(&inv)?;
  Ok(inv)
}

/// Accept an invitation as an authenticated user → create the membership (accounts.md §4).
pub async fn accept_invitation(pool: &db::PgPool, invitation_id: &InvitationId, user_id: &UserId) -> ApiResult<()> {
  let inv = invitations::find_invitation_by_id(pool, invitation_id)
    .await?
    .ok_or_else(|| ApiError::NotFound("invitation not found".into()))?;
  ensure_invitation_open(&inv)?;

  let mut tx = pool.begin().await?;
  let marked = invitations::mark_invitation_accepted(&mut *tx, &inv.id, user_id).await?;
  if marked == 0 {
    return Err(ApiError::BadRequest("invitation is no longer acceptable".into()));
  }
  // Idempotent membership: skip if it already exists.
  if members::find_membership(&mut *tx, &inv.account_id, user_id).await?.is_none() {
    members::create_membership(&mut *tx, &inv.account_id, user_id, &inv.role).await?;
  }
  tx.commit().await?;
  Ok(())
}

/* ── helpers ── */

fn ensure_invitation_open(inv: &Invitation) -> ApiResult<()> {
  if inv.revoked_at.is_some() {
    return Err(ApiError::BadRequest("invitation was revoked".into()));
  }
  if inv.accepted_at.is_some() {
    return Err(ApiError::BadRequest("invitation was already accepted".into()));
  }
  if inv.expires_at <= Utc::now() {
    return Err(ApiError::BadRequest("invitation has expired".into()));
  }
  Ok(())
}

/// Create a session row for an existing user and mint its token.
async fn start_session(
  pool: &db::PgPool,
  jwt: &JwtKeys,
  user: User,
  impersonator_user_id: Option<&UserId>,
  meta: &ReqMeta,
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
    },
  )
  .await?;
  let token = jwt
    .mint(session.id.as_str(), user.id.as_str(), user.session_version)
    .map_err(ApiError::Internal)?;
  Ok(AuthOutcome { user, session, token })
}

/// Create a session inside an open transaction (signup flows).
async fn new_session(
  tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
  user_id: &UserId,
  meta: &ReqMeta,
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
    },
  )
  .await?;
  Ok(session)
}

fn sha256_hex(input: &str) -> String {
  let mut hasher = Sha256::new();
  hasher.update(input.as_bytes());
  hex::encode(hasher.finalize())
}

/// A short, human-friendly invite code like `7F3K-QT9M` (Crockford-ish, no ambiguous chars).
fn gen_code() -> String {
  use rand::Rng;
  const ALPHABET: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
  let mut rng = rand::thread_rng();
  let pick = |rng: &mut rand::rngs::ThreadRng| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char;
  let g1: String = (0..4).map(|_| pick(&mut rng)).collect();
  let g2: String = (0..4).map(|_| pick(&mut rng)).collect();
  format!("{g1}-{g2}")
}

/// A 32-hex-char magic-link token (128 bits).
fn gen_token() -> String {
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
