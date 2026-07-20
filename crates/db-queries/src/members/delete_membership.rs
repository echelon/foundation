use ids::{AccountId, UserId};
use sqlx::PgExecutor;

/// Remove a membership. The caller enforces the role rules (owner can't be removed, etc.) and
/// writes the `member_removed` audit rows in the same transaction (audit_logs.md §3).
/// Returns rows affected (0 = wasn't a member).
pub async fn delete_membership<'e, E: PgExecutor<'e>>(
  exec: E,
  account_id: &AccountId,
  user_id: &UserId,
) -> sqlx::Result<u64> {
  let result = sqlx::query!(
    r#"DELETE FROM account_members WHERE account_id = $1 AND user_id = $2"#,
    account_id.as_str(),
    user_id.as_str(),
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected())
}
