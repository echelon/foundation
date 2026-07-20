use ids::AccountId;
use sqlx::PgExecutor;

/// Revoke any still-open invitations for `email` on this account before issuing a fresh one
/// (accounts.md §4 — re-inviting supersedes the prior invite). Returns rows affected.
pub async fn revoke_outstanding_invitations<'e, E: PgExecutor<'e>>(
  exec: E,
  account_id: &AccountId,
  email: &str,
) -> sqlx::Result<u64> {
  let result = sqlx::query!(
    r#"UPDATE invitations SET revoked_at = now()
       WHERE account_id = $1 AND email = $2
         AND accepted_at IS NULL AND revoked_at IS NULL"#,
    account_id.as_str(),
    email,
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected())
}
