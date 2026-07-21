use ids::{AccountId, InvitationId};
use sqlx::PgExecutor;

/// Revoke any still-open invitations for `email` on this account before issuing a fresh one
/// (accounts.md §4 — re-inviting supersedes the prior invite). Returns the revoked ids so
/// the caller can write an `invitation_revoked` audit row for each.
pub async fn revoke_outstanding_invitations<'e, E: PgExecutor<'e>>(
  exec: E,
  account_id: &AccountId,
  email: &str,
) -> sqlx::Result<Vec<InvitationId>> {
  let rows = sqlx::query!(
    r#"UPDATE invitations SET revoked_at = now()
       WHERE account_id = $1 AND email = $2
         AND accepted_at IS NULL AND revoked_at IS NULL
       RETURNING id AS "id: InvitationId""#,
    account_id.as_str(),
    email,
  )
  .fetch_all(exec)
  .await?;
  Ok(rows.into_iter().map(|r| r.id).collect())
}
