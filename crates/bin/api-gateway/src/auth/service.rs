//! Auth flow logic (accounts.md §3–§5, auth.md §2). Handlers are thin wrappers over these.
//! The signup/SSO-signup flows run in ONE transaction (user + account + owner membership +
//! session commit atomically — accounts.md §3).

use chrono::Utc;
use sha2::{Digest, Sha256};

use db_queries::{
  account_audit_logs, accounts, email_changes, email_verifications, invitations, members, password_resets, sessions,
  sso, user_audit_logs, user_avatars, users, Account, Invitation, Session, User,
};
use enums::AuditEvent;
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

/// Everything `signup` needs beyond connections (named fields — the email/password/name run
/// of `&str`s is exactly the transposition hazard the args-struct rule exists for).
pub struct SignupArgs<'a> {
  pub email: &'a str,
  pub password: &'a str,
  pub display_name: Option<&'a str>,
  /// An invitation code or magic-link token to redeem during signup (accounts.md §4 flow 1).
  /// When set, the email is LOCKED to the invite's email, and the new user gets a membership
  /// in the inviting account INSTEAD of a personal organization (invites never create
  /// accounts — only memberships).
  pub invitation: Option<&'a str>,
}

/// Email + password signup (accounts.md §3A), including the signup-as-login rule and the
/// invite-redemption flows (accounts.md §4).
pub async fn signup(pool: &db::PgPool, jwt: &JwtKeys, args: SignupArgs<'_>, meta: &ReqMeta) -> ApiResult<AuthOutcome> {
  if args.password.len() < MIN_PASSWORD_LEN {
    return Err(ApiError::BadRequest(format!("password must be at least {MIN_PASSWORD_LEN} characters")));
  }
  let email_canon = canonicalize(args.email);

  // Resolve the invitation up front so a dead invite fails before any user is created, and
  // enforce the email lock (the join page pre-fills and locks the field; the API is the
  // actual gate).
  let invitation = match args.invitation {
    Some(code_or_token) => {
      let inv = resolve_invitation(pool, code_or_token).await?;
      if inv.email != email_canon {
        return Err(ApiError::BadRequest("this invitation was issued to a different email address".into()));
      }
      Some(inv)
    }
    None => None,
  };

  // Existing user? signup-as-login when the password is correct; otherwise route to login.
  if let Some(user) = users::find_user_by_email(pool, &email_canon).await? {
    match &user.password_hash {
      Some(hash) if verify_password(args.password, hash) => {
        // Flow 2 (accounts.md §4): an existing user redeeming an invite — accept, then log in.
        if let Some(inv) = &invitation {
          let mut tx = pool.begin().await?;
          accept_open_invitation(&mut tx, inv, &user.id, meta.ip.as_deref()).await?;
          tx.commit().await?;
        }
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
  let hash = hash_password(args.password).map_err(ApiError::Internal)?;
  let mut tx = pool.begin().await?;

  let user = users::create_user(
    &mut *tx,
    users::NewUser {
      email: email_canon,
      email_clean: canonicalize_clean(args.email),
      email_original: args.email.trim().to_string(),
      password_hash: Some(hash),
      display_name: args.display_name.map(str::to_string),
      email_verified_at: None,
    },
  )
  .await?;
  match &invitation {
    // Normal signup: the user's own organization, with them as owner.
    None => {
      let account = accounts::create_account(
        &mut *tx,
        accounts::NewAccount { name: None, placeholder_business_name: &crate::business_names::random_business_name() },
      )
      .await?;
      members::create_membership(&mut *tx, &account.id, &user.id, "owner").await?;
      provision_new_user(&mut tx, &user, &account, meta).await?;
    }
    // Invited signup (flow 1): membership in the INVITING account only — no personal org.
    // The accept writes the member_added/invitation_accepted audit rows; account_created
    // would be a lie here, so provisioning is just the avatar.
    Some(inv) => {
      create_default_avatar(&mut tx, &user).await?;
      accept_open_invitation(&mut tx, inv, &user.id, meta.ip.as_deref()).await?;
    }
  }
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
  let account = accounts::create_account(
    &mut *tx,
    accounts::NewAccount { name: None, placeholder_business_name: &crate::business_names::random_business_name() },
  )
  .await?;
  members::create_membership(&mut *tx, &account.id, &user.id, "owner").await?;
  provision_new_user(&mut tx, &user, &account, meta).await?;
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
pub async fn change_password(
  pool: &db::PgPool,
  jwt: &JwtKeys,
  user: &User,
  current_password: &str,
  new_password: &str,
  logout_others: bool,
  current_session_id: &SessionId,
  ip: Option<&str>,
) -> ApiResult<SessionReminted> {
  if new_password.len() < MIN_PASSWORD_LEN {
    return Err(ApiError::BadRequest(format!("password must be at least {MIN_PASSWORD_LEN} characters")));
  }
  // Fresh re-auth: confirm the current password.
  let current_hash = user
    .password_hash
    .as_deref()
    .ok_or_else(|| ApiError::BadRequest("set a password via reset before changing it".into()))?;
  if !verify_password(current_password, current_hash) {
    return Err(ApiError::Unauthorized("current password is incorrect".into()));
  }

  let new_hash = hash_password(new_password).map_err(ApiError::Internal)?;
  let mut tx = pool.begin().await?;
  users::set_password_hash(&mut *tx, &user.id, &new_hash).await?;
  write_user_event(&mut *tx, &user.id, AuditEvent::PasswordChanged, None, ip).await?;

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

/// Change the user's display name (nullable — empty clears it). Atomically: update `users`,
/// refresh the avatar's person-letter monogram, and write the ORG-VISIBLE
/// `display_name_changed` audit event (audit_logs.md §3 — accounts the user belongs to can see
/// it). Returns the new name.
pub async fn change_display_name(
  pool: &db::PgPool,
  user: &User,
  new_name: Option<&str>,
  ip: Option<&str>,
) -> ApiResult<Option<String>> {
  let new_name = new_name.map(str::trim).filter(|s| !s.is_empty());
  if let Some(name) = new_name {
    if name.len() > 100 {
      return Err(ApiError::BadRequest("name must be 100 characters or fewer".into()));
    }
  }
  if new_name == user.display_name.as_deref() {
    return Ok(user.display_name.clone()); // no-op; no audit noise
  }

  let (first, last) = person_letters(new_name);
  let mut tx = pool.begin().await?;
  users::update_display_name(&mut *tx, &user.id, new_name).await?;
  user_avatars::set_person_letters(&mut *tx, &user.id, first.as_deref(), last.as_deref()).await?;
  let data = serde_json::json!({ "from": user.display_name, "to": new_name });
  write_user_event(&mut *tx, &user.id, AuditEvent::DisplayNameChanged, Some(data), ip).await?;
  tx.commit().await?;
  Ok(new_name.map(str::to_string))
}

/// First letter of the first word and first letter of the last word, uppercased — the
/// person-name monogram (accounts.md §2).
fn person_letters(name: Option<&str>) -> (Option<String>, Option<String>) {
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
  .await
  .map_err(|e| match &e {
    // The partial unique index (one live invitation per account+email) caught a concurrent
    // create that this transaction's revoke-then-insert couldn't see.
    sqlx::Error::Database(db) if db.constraint() == Some("invitations_one_live_per_account_email") => {
      ApiError::Conflict("an invitation for this email is already outstanding".into())
    }
    _ => ApiError::from(e),
  })?;
  // invitation_sent — account log (audit_logs.md §3), same transaction.
  account_audit_logs::create_account_audit_log(
    &mut *tx,
    account_audit_logs::NewAccountAuditLog {
      account_id,
      actor_user_id: Some(inviter_user_id),
      user_id: None,
      event: AuditEvent::InvitationSent,
      target_type: Some("invitation"),
      target_id: Some(invitation.id.as_str()),
      data: Some(serde_json::json!({ "email": email_canon, "role": role })),
      ip: None,
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

/// An open invitation enriched with what the join page needs to branch (accounts.md §4):
/// the account's name for "join {account} as {role}?", and whether a user already exists for
/// the invited email (existing user → login screen; no user → locked-email signup).
pub struct ResolvedInvitation {
  pub invitation: Invitation,
  pub account_name: Option<String>,
  pub invitee_user_exists: bool,
}

/// [`resolve_invitation`] plus the join-page context.
pub async fn resolve_invitation_for_join_page(
  pool: &db::PgPool,
  code_or_token: &str,
) -> ApiResult<ResolvedInvitation> {
  let invitation = resolve_invitation(pool, code_or_token).await?;
  // The business display name: the set name, else the placeholder (never the bare `name`,
  // which is null for every self-serve account).
  let account_name = accounts::find_account_by_id(pool, &invitation.account_id)
    .await?
    .map(|a| a.business_name.unwrap_or(a.placeholder_business_name));
  let invitee_user_exists = users::find_user_by_email(pool, &invitation.email).await?.is_some();
  Ok(ResolvedInvitation { invitation, account_name, invitee_user_exists })
}

/// Accept an invitation as an authenticated user → create the membership (accounts.md §4).
pub async fn accept_invitation(pool: &db::PgPool, invitation_id: &InvitationId, user_id: &UserId) -> ApiResult<()> {
  let inv = invitations::find_invitation_by_id(pool, invitation_id)
    .await?
    .ok_or_else(|| ApiError::NotFound("invitation not found".into()))?;
  ensure_invitation_open(&inv)?;

  let mut tx = pool.begin().await?;
  accept_open_invitation(&mut tx, &inv, user_id, None).await?;
  tx.commit().await?;
  Ok(())
}

/// The accept itself, composable into a larger transaction (plain accept, or invited signup):
/// atomically claim the invitation, create the membership (idempotent — the user may already
/// be a member via another route), and write the audit rows.
async fn accept_open_invitation(
  tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
  inv: &Invitation,
  user_id: &UserId,
  ip: Option<&str>,
) -> ApiResult<()> {
  let marked = invitations::mark_invitation_accepted(&mut **tx, &inv.id, user_id).await?;
  if marked == 0 {
    return Err(ApiError::BadRequest("invitation is no longer acceptable".into()));
  }
  // Idempotent membership: skip if it already exists.
  if members::find_membership(&mut **tx, &inv.account_id, user_id).await?.is_none() {
    members::create_membership(&mut **tx, &inv.account_id, user_id, &inv.role).await?;
    // member_added — U-row for the new member + A-row (audit_logs.md §3).
    write_user_event(
      &mut **tx,
      user_id,
      AuditEvent::MemberAdded,
      Some(serde_json::json!({ "account_id": inv.account_id, "role": inv.role })),
      ip,
    )
    .await?;
    account_audit_logs::create_account_audit_log(
      &mut **tx,
      account_audit_logs::NewAccountAuditLog {
        account_id: &inv.account_id,
        actor_user_id: Some(user_id),
        user_id: Some(user_id),
        event: AuditEvent::MemberAdded,
        target_type: None,
        target_id: None,
        data: Some(serde_json::json!({ "role": inv.role })),
        ip,
      },
    )
    .await?;
  }
  // invitation_accepted — account log, alongside member_added (audit_logs.md §3).
  account_audit_logs::create_account_audit_log(
    &mut **tx,
    account_audit_logs::NewAccountAuditLog {
      account_id: &inv.account_id,
      actor_user_id: Some(user_id),
      user_id: Some(user_id),
      event: AuditEvent::InvitationAccepted,
      target_type: Some("invitation"),
      target_id: Some(inv.id.as_str()),
      data: None,
      ip,
    },
  )
  .await?;
  Ok(())
}

/// Set or clear the business display name. Owner/admin only (authorized by the handler);
/// `None` clears it, falling back to `placeholder_business_name` in clients. Writes
/// `business_name_changed` to the account audit log in the update's transaction.
pub async fn update_business_name(
  pool: &db::PgPool,
  account_id: &AccountId,
  actor_user_id: &UserId,
  business_name: Option<&str>,
  ip: Option<&str>,
) -> ApiResult<Account> {
  let mut tx = pool.begin().await?;
  // Read the outgoing value inside the tx so the audit row's before/after can't race a
  // concurrent change ({"from", "to"} — the display_name_changed convention).
  let before = accounts::find_account_by_id(&mut *tx, account_id)
    .await?
    .ok_or_else(|| ApiError::NotFound("account not found".into()))?;
  let account = accounts::set_business_name(&mut *tx, account_id, business_name)
    .await?
    .ok_or_else(|| ApiError::NotFound("account not found".into()))?;
  account_audit_logs::create_account_audit_log(
    &mut *tx,
    account_audit_logs::NewAccountAuditLog {
      account_id,
      actor_user_id: Some(actor_user_id),
      user_id: None,
      event: AuditEvent::BusinessNameChanged,
      target_type: None,
      target_id: None,
      data: Some(serde_json::json!({ "from": before.business_name, "to": account.business_name })),
      ip,
    },
  )
  .await?;
  tx.commit().await?;
  Ok(account)
}

/// Remove a member from an account (accounts.md §4, auth.md §4). The handler has already
/// verified the caller is owner/admin; this enforces the finer rules:
///   - the target must currently be a member (404),
///   - nobody removes themselves (leave ≠ remove),
///   - the owner can never be removed,
///   - an admin can only remove `member`s — outranking removals need the owner.
/// Writes `member_removed` U+A audit rows in the removal transaction.
pub async fn remove_member(
  pool: &db::PgPool,
  actor: &User,
  actor_role: &str,
  account_id: &AccountId,
  target_user_id: &UserId,
  ip: Option<&str>,
) -> ApiResult<()> {
  if actor.id == *target_user_id {
    return Err(ApiError::BadRequest("you can't remove yourself from the organization".into()));
  }
  let target = members::find_membership(pool, account_id, target_user_id)
    .await?
    .ok_or_else(|| ApiError::NotFound("that user is not a member of this organization".into()))?;
  if target.role == "owner" {
    return Err(ApiError::BadRequest("the owner can't be removed".into()));
  }
  if actor_role != "owner" && target.role != "member" {
    return Err(ApiError::Forbidden("only the owner can remove an admin".into()));
  }

  let mut tx = pool.begin().await?;
  members::delete_membership(&mut *tx, account_id, target_user_id).await?;
  let data = serde_json::json!({ "account_id": account_id, "role": target.role });
  user_audit_logs::create_user_audit_log(
    &mut *tx,
    user_audit_logs::NewUserAuditLog {
      user_id: target_user_id,
      actor_user_id: Some(&actor.id),
      event: AuditEvent::MemberRemoved,
      target_type: None,
      target_id: None,
      data: Some(data),
      ip,
    },
  )
  .await?;
  account_audit_logs::create_account_audit_log(
    &mut *tx,
    account_audit_logs::NewAccountAuditLog {
      account_id,
      actor_user_id: Some(&actor.id),
      user_id: Some(target_user_id),
      event: AuditEvent::MemberRemoved,
      target_type: None,
      target_id: None,
      data: Some(serde_json::json!({ "role": target.role })),
      ip,
    },
  )
  .await?;
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

/// Provision the non-essential-but-standard bits of a new user, inside the signup transaction:
/// the avatar row (with a computed gravatar hash + letters, left UNPOLLED so the gravatar-job
/// picks it up) and the `account_created` audit events (U + A — accounts.md, audit_logs.md §3).
async fn provision_new_user(
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
async fn create_default_avatar(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, user: &User) -> ApiResult<()> {
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
async fn write_user_event<'e, E: sqlx::PgExecutor<'e>>(
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
fn email_letters(email: &str) -> (Option<String>, Option<String>) {
  let local = email.split('@').next().unwrap_or("");
  let chars: Vec<char> = local.chars().filter(|c| c.is_alphanumeric()).collect();
  let first = chars.first().map(|c| c.to_uppercase().to_string());
  let last = chars.last().map(|c| c.to_uppercase().to_string());
  (first, last)
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
