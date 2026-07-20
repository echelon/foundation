//! Authorization helpers (auth.md §4). Roles gate account-scoped actions.

use db_queries::{members, AccountMember};

use crate::error::{ApiError, ApiResult};

/// Require that `user_id` is a member of `account_id` with one of `allowed_roles`. Returns the
/// membership on success; `403` if the role is insufficient, `404` if there is no membership
/// (we don't confirm the account exists to a non-member).
pub async fn require_account_role(
  pool: &db::PgPool,
  user_id: &str,
  account_id: &str,
  allowed_roles: &[&str],
) -> ApiResult<AccountMember> {
  let membership = members::find_membership(pool, account_id, user_id)
    .await?
    .ok_or_else(|| ApiError::NotFound("account not found".into()))?;

  if allowed_roles.contains(&membership.role.as_str()) {
    Ok(membership)
  } else {
    Err(ApiError::Forbidden(format!(
      "requires one of [{}]; you are '{}'",
      allowed_roles.join(", "),
      membership.role
    )))
  }
}
