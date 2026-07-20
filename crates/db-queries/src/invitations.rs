//! `invitations` queries (accounts.md §2, §4). Code AND magic link are two doors to the same
//! invitation; invites grant `admin`/`member`, never `owner`. Invites create memberships, not
//! accounts.

use chrono::{DateTime, Utc};
use sqlx::PgExecutor;

use crate::rows::Invitation;

const COLS: &str = "id, account_id, invited_by_user_id, accepted_by_user_id, email, role, code, \
                    token_hash, created_at, expires_at, accepted_at, revoked_at";

#[derive(Debug, Clone)]
pub struct NewInvitation<'a> {
  pub account_id: &'a str,
  pub invited_by_user_id: &'a str,
  /// Canonicalized invitee email (the invite is bound to it).
  pub email: &'a str,
  /// `admin` | `member` (never `owner`).
  pub role: &'a str,
  /// Short human code (e.g. `7F3K-QT9M`).
  pub code: &'a str,
  /// sha256 hex of the magic-link token (the token itself is emailed, never stored).
  pub token_hash: &'a str,
  pub expires_at: DateTime<Utc>,
}

pub async fn create_invitation<'e, E: PgExecutor<'e>>(exec: E, new: NewInvitation<'_>) -> sqlx::Result<Invitation> {
  let id = ids::new_id(ids::prefix::INVITATION);
  let sql = format!(
    "INSERT INTO invitations (id, account_id, invited_by_user_id, email, role, code, token_hash, expires_at) \
     VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING {COLS}"
  );
  sqlx::query_as::<_, Invitation>(&sql)
    .bind(id)
    .bind(new.account_id)
    .bind(new.invited_by_user_id)
    .bind(new.email)
    .bind(new.role)
    .bind(new.code)
    .bind(new.token_hash)
    .bind(new.expires_at)
    .fetch_one(exec)
    .await
}

pub async fn find_by_id<'e, E: PgExecutor<'e>>(exec: E, id: &str) -> sqlx::Result<Option<Invitation>> {
  let sql = format!("SELECT {COLS} FROM invitations WHERE id = $1");
  sqlx::query_as::<_, Invitation>(&sql).bind(id).fetch_optional(exec).await
}

pub async fn find_by_code<'e, E: PgExecutor<'e>>(exec: E, code: &str) -> sqlx::Result<Option<Invitation>> {
  let sql = format!("SELECT {COLS} FROM invitations WHERE code = $1");
  sqlx::query_as::<_, Invitation>(&sql).bind(code).fetch_optional(exec).await
}

pub async fn find_by_token_hash<'e, E: PgExecutor<'e>>(exec: E, token_hash: &str) -> sqlx::Result<Option<Invitation>> {
  let sql = format!("SELECT {COLS} FROM invitations WHERE token_hash = $1");
  sqlx::query_as::<_, Invitation>(&sql).bind(token_hash).fetch_optional(exec).await
}

/// Mark an invitation accepted by a user. Returns rows affected (0 if already accepted/revoked).
pub async fn mark_accepted<'e, E: PgExecutor<'e>>(exec: E, id: &str, accepted_by_user_id: &str) -> sqlx::Result<u64> {
  let r = sqlx::query(
    "UPDATE invitations SET accepted_by_user_id = $2, accepted_at = now() \
     WHERE id = $1 AND accepted_at IS NULL AND revoked_at IS NULL",
  )
  .bind(id)
  .bind(accepted_by_user_id)
  .execute(exec)
  .await?;
  Ok(r.rows_affected())
}

/// Revoke any outstanding (unaccepted) invitations for an email on an account — re-inviting
/// revokes the prior one (accounts.md §4 controller rule).
pub async fn revoke_outstanding_for_email<'e, E: PgExecutor<'e>>(
  exec: E,
  account_id: &str,
  email: &str,
) -> sqlx::Result<u64> {
  let r = sqlx::query(
    "UPDATE invitations SET revoked_at = now() \
     WHERE account_id = $1 AND email = $2 AND accepted_at IS NULL AND revoked_at IS NULL",
  )
  .bind(account_id)
  .bind(email)
  .execute(exec)
  .await?;
  Ok(r.rows_affected())
}
