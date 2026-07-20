use ids::{InvitationId, UserId};
use sqlx::PgExecutor;

/// Atomically claim an invitation: set `accepted_at`/`accepted_by_user_id` only if still open
/// (not accepted, not revoked, not expired). Returns rows affected — 0 means it was no longer
/// acceptable and the caller should reject.
pub async fn mark_invitation_accepted<'e, E: PgExecutor<'e>>(
  exec: E,
  id: &InvitationId,
  accepted_by: &UserId,
) -> sqlx::Result<u64> {
  let result = sqlx::query!(
    r#"UPDATE invitations
       SET accepted_at = now(), accepted_by_user_id = $2
       WHERE id = $1 AND accepted_at IS NULL AND revoked_at IS NULL AND expires_at > now()"#,
    id.as_str(),
    accepted_by.as_str(),
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected())
}
