//! Organization invitations: create, resolve, accept, cancel (accounts.md §4).

use chrono::Utc;

use db_queries::{
  account_audit_logs, accounts, invitations, members, users, Invitation,
};
use enums::AuditEvent;
use ids::{AccountId, InvitationId, UserId};

use crate::auth::email::canonicalize;
use crate::error::{ApiError, ApiResult};


#[allow(unused_imports)]
use super::*;

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
  role: enums::MemberRole,
) -> ApiResult<CreatedInvitation> {
  if role == enums::MemberRole::Owner {
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
  // Re-inviting supersedes any outstanding invitation — and every supersede is audited, so
  // the org log tells the whole story, not just the fresh send.
  let superseded = invitations::revoke_outstanding_invitations(&mut *tx, account_id, &email_canon).await?;
  for old_id in &superseded {
    account_audit_logs::create_account_audit_log(
      &mut *tx,
      account_audit_logs::NewAccountAuditLog {
        account_id,
        actor_user_id: Some(inviter_user_id),
        user_id: None,
        event: AuditEvent::InvitationRevoked,
        target_type: Some("invitation"),
        target_id: Some(old_id.as_str()),
        data: Some(serde_json::json!({ "email": email_canon, "superseded": true })),
        ip: None,
      },
    )
    .await?;
  }

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

/// Cancel a pending invitation (owner/admin, authorized by the handler). 404 for an unknown,
/// foreign, or already-closed invitation. Writes `invitation_revoked` in the same tx.
pub async fn cancel_invitation(
  pool: &db::PgPool,
  account_id: &AccountId,
  actor_user_id: &UserId,
  invitation_id: &InvitationId,
  ip: Option<&str>,
) -> ApiResult<()> {
  let inv = invitations::find_invitation_by_id(pool, invitation_id)
    .await?
    .filter(|i| &i.account_id == account_id)
    .ok_or_else(|| ApiError::NotFound("invitation not found".into()))?;

  let mut tx = pool.begin().await?;
  let revoked = invitations::revoke_invitation(&mut *tx, account_id, invitation_id).await?;
  if revoked == 0 {
    return Err(ApiError::NotFound("invitation is no longer pending".into()));
  }
  account_audit_logs::create_account_audit_log(
    &mut *tx,
    account_audit_logs::NewAccountAuditLog {
      account_id,
      actor_user_id: Some(actor_user_id),
      user_id: None,
      event: AuditEvent::InvitationRevoked,
      target_type: Some("invitation"),
      target_id: Some(invitation_id.as_str()),
      data: Some(serde_json::json!({ "email": inv.email })),
      ip,
    },
  )
  .await?;
  tx.commit().await?;
  Ok(())
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
pub(crate) async fn accept_open_invitation(
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
    members::create_membership(&mut **tx, &inv.account_id, user_id, inv.role).await?;
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

pub(crate) fn ensure_invitation_open(inv: &Invitation) -> ApiResult<()> {
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

/// A short, human-friendly invite code like `7F3K-QT9M` (Crockford-ish, no ambiguous chars).
pub(crate) fn gen_code() -> String {
  use rand::Rng;
  const ALPHABET: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
  let mut rng = rand::thread_rng();
  let pick = |rng: &mut rand::rngs::ThreadRng| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char;
  let g1: String = (0..4).map(|_| pick(&mut rng)).collect();
  let g2: String = (0..4).map(|_| pick(&mut rng)).collect();
  format!("{g1}-{g2}")
}
