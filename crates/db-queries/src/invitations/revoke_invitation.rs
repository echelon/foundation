use ids::{AccountId, InvitationId};
use sqlx::PgExecutor;

/// Cancel one open invitation — scoped by account so a foreign invitation id can never be
/// touched. Returns rows affected (0 = unknown/foreign/already closed).
pub async fn revoke_invitation<'e, E: PgExecutor<'e>>(
  exec: E,
  account_id: &AccountId,
  id: &InvitationId,
) -> sqlx::Result<u64> {
  let result = sqlx::query!(
    r#"UPDATE invitations SET revoked_at = now()
       WHERE id = $1 AND account_id = $2 AND accepted_at IS NULL AND revoked_at IS NULL"#,
    id.as_str(),
    account_id.as_str(),
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected())
}
