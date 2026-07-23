//! Account business-name updates.


use db_queries::{
  account_audit_logs, accounts, Account,
};
use enums::AuditEvent;
use ids::{AccountId, UserId};

use crate::error::{ApiError, ApiResult};


#[allow(unused_imports)]
use super::*;

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
      data: Some(crate::audit_data::AuditData { from: before.business_name.clone(), to: account.business_name.clone(), ..Default::default() }.value()),
      ip,
    },
  )
  .await?;
  tx.commit().await?;
  Ok(account)
}
