//! SSO: OAuth login/signup and linking/unlinking providers (auth.md §2, §5).

use chrono::Utc;

use db_queries::{
  accounts, members,
  sso, user_audit_logs, users, User,
};
use enums::{AuditEvent, SessionCreationType};

use crate::auth::email::{canonicalize, canonicalize_clean};
use crate::auth::token::JwtKeys;
use crate::error::{ApiError, ApiResult};


#[allow(unused_imports)]
use super::*;

/// A verified SSO identity (named fields — the provider/sub/email `&str` run is exactly the
/// transposition hazard the args-struct rule exists for). The caller (the handler) has
/// already VERIFIED this identity cryptographically — nothing here is client-trusted.
pub struct SsoLoginArgs<'a> {
  pub provider: &'a str,
  /// The provider's stable subject (`sub`) — the identity anchor, never the email.
  pub provider_user_id: &'a str,
  pub email: &'a str,
  pub display_name: Option<&'a str>,
}

/// SSO login/signup (accounts.md §3B/C, §5). Resolves on `(provider, provider_user_id)` —
/// NEVER on email. A matching email with no existing connection is a hard stop (no silent
/// merge — prevents takeover).
pub async fn oauth_login(pool: &db::PgPool, jwt: &JwtKeys, args: SsoLoginArgs<'_>, meta: &ReqMeta) -> ApiResult<AuthOutcome> {
  let SsoLoginArgs { provider, provider_user_id, email, display_name } = args;
  if provider != "google" && provider != "github" {
    return Err(ApiError::BadRequest(format!("unsupported provider: {provider}")));
  }

  // 1. Existing connection → log in.
  if let Some(conn) = sso::find_sso_by_provider_sub(pool, provider, provider_user_id).await? {
    sso::touch_sso_last_used(pool, &conn.id).await?;
    let user = users::find_user_by_id(pool, &conn.user_id)
      .await?
      .ok_or_else(|| ApiError::Internal("sso connection references missing user".into()))?;
    return start_session(pool, jwt, user, None, meta, SessionCreationType::SsoLogin).await;
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
  members::create_membership(&mut *tx, &account.id, &user.id, enums::MemberRole::Owner).await?;
  provision_new_user(&mut tx, &user, &account, meta).await?;
  let session = new_session(&mut tx, &user.id, meta, SessionCreationType::SsoSignup).await?;
  tx.commit().await?;

  let token = jwt
    .mint(session.id.as_str(), user.id.as_str(), user.session_version)
    .map_err(ApiError::Internal)?;
  Ok(AuthOutcome { user, session, token })
}

/// Link a VERIFIED SSO identity to the CURRENT user (accounts.md §5 flow 1/3). Connections
/// are never moved implicitly: (provider, sub) already linked to a different user is a hard
/// 409. Idempotent for the same user. Audited (`sso_connected`, org-visible).
pub async fn link_sso(pool: &db::PgPool, user: &User, args: SsoLoginArgs<'_>, ip: Option<&str>) -> ApiResult<()> {
  if let Some(existing) = sso::find_sso_by_provider_sub(pool, args.provider, args.provider_user_id).await? {
    if existing.user_id == user.id {
      return Ok(()); // already linked here — nothing to do
    }
    return Err(ApiError::Conflict("this account is already connected to a different user".into()));
  }
  let mut tx = pool.begin().await?;
  let conn = sso::create_sso_connection(
    &mut *tx,
    sso::NewSsoConnection {
      user_id: &user.id,
      provider: args.provider,
      provider_user_id: args.provider_user_id,
      provider_email: &canonicalize(args.email),
      provider_user_display_name: args.display_name,
    },
  )
  .await?;
  user_audit_logs::create_user_audit_log(
    &mut *tx,
    user_audit_logs::NewUserAuditLog {
      user_id: &user.id,
      actor_user_id: Some(&user.id),
      event: AuditEvent::SsoConnected,
      target_type: Some("sso_connection"),
      target_id: Some(conn.id.as_str()),
      data: Some(crate::audit_data::AuditData { provider: Some(args.provider.to_string()), ..Default::default() }.value()),
      ip,
    },
  )
  .await?;
  tx.commit().await?;
  Ok(())
}

/// Disconnect an SSO connection (accounts.md §5, removal rule per bt 2026-07-21: allowed
/// when the user has a password OR a verified email — either leaves them a way back in.
/// Neither = removal would risk a lockout, so it's refused). 404 for unknown/foreign ids.
pub async fn unlink_sso(
  pool: &db::PgPool,
  user: &User,
  sso_id: &ids::SsoConnectionId,
  ip: Option<&str>,
) -> ApiResult<()> {
  let connections = sso::list_sso_for_user(pool, &user.id).await?;
  let conn = connections
    .iter()
    .find(|c| &c.id == sso_id)
    .ok_or_else(|| ApiError::NotFound("connection not found".into()))?;

  if user.password_hash.is_none() && user.email_verified_at.is_none() {
    return Err(ApiError::Forbidden(
      "set a password or verify your email first — removing this connection could lock you out".into(),
    ));
  }

  let mut tx = pool.begin().await?;
  let deleted = sso::delete_sso_connection(&mut *tx, &user.id, sso_id).await?;
  if deleted == 0 {
    return Err(ApiError::NotFound("connection not found".into()));
  }
  user_audit_logs::create_user_audit_log(
    &mut *tx,
    user_audit_logs::NewUserAuditLog {
      user_id: &user.id,
      actor_user_id: Some(&user.id),
      event: AuditEvent::SsoDisconnected,
      target_type: Some("sso_connection"),
      target_id: Some(sso_id.as_str()),
      data: Some(crate::audit_data::AuditData { provider: Some(conn.provider.to_string()), ..Default::default() }.value()),
      ip,
    },
  )
  .await?;
  tx.commit().await?;
  Ok(())
}
