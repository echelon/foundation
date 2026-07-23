//! Membership removal (auth.md §4 role rules).


use db_queries::{
  account_audit_logs, members, user_audit_logs, User,
};
use enums::AuditEvent;
use ids::{AccountId, UserId};

use crate::error::{ApiError, ApiResult};


#[allow(unused_imports)]
use super::*;

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
  actor_role: enums::MemberRole,
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
  if target.role == enums::MemberRole::Owner {
    return Err(ApiError::BadRequest("the owner can't be removed".into()));
  }
  if actor_role != enums::MemberRole::Owner && target.role != enums::MemberRole::Member {
    return Err(ApiError::Forbidden("only the owner can remove an admin".into()));
  }

  let mut tx = pool.begin().await?;
  members::delete_membership(&mut *tx, account_id, target_user_id).await?;
  let data = crate::audit_data::AuditData { account_id: Some(account_id.clone()), role: Some(target.role), ..Default::default() }.value();
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
      data: Some(crate::audit_data::AuditData { role: Some(target.role), ..Default::default() }.value()),
      ip,
    },
  )
  .await?;
  tx.commit().await?;
  Ok(())
}

/* ── helpers ── */
