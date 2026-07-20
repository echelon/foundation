use chrono::{DateTime, Utc};
use ids::{AccountId, InvitationId, UserId};
use sqlx::PgExecutor;

use super::Invitation;

#[derive(Debug, Clone)]
pub struct NewInvitation<'a> {
  pub account_id: &'a AccountId,
  pub invited_by_user_id: &'a UserId,
  pub email: &'a str,
  /// CHECK-constrained to `admin`/`member`.
  pub role: &'a str,
  pub code: &'a str,
  pub token_hash: &'a str,
  pub expires_at: DateTime<Utc>,
}

pub async fn create_invitation<'e, E: PgExecutor<'e>>(
  exec: E,
  new: NewInvitation<'_>,
) -> sqlx::Result<Invitation> {
  let id = InvitationId::generate();
  sqlx::query_as!(
    Invitation,
    r#"INSERT INTO invitations
         (id, account_id, invited_by_user_id, email, role, code, token_hash, expires_at)
       VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
       RETURNING id AS "id: InvitationId", account_id AS "account_id: AccountId",
                 invited_by_user_id AS "invited_by_user_id: UserId",
                 accepted_by_user_id AS "accepted_by_user_id?: UserId",
                 email, role, code, token_hash,
                 created_at, expires_at, accepted_at, revoked_at"#,
    id.as_str(),
    new.account_id.as_str(),
    new.invited_by_user_id.as_str(),
    new.email,
    new.role,
    new.code,
    new.token_hash,
    new.expires_at,
  )
  .fetch_one(exec)
  .await
}
