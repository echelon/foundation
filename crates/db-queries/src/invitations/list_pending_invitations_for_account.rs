use ids::AccountId;
use sqlx::PgExecutor;

use super::Invitation;

/// Open invitations for the org page: not accepted, not revoked, not expired — newest first.
pub async fn list_pending_invitations_for_account<'e, E: PgExecutor<'e>>(
  exec: E,
  account_id: &AccountId,
) -> sqlx::Result<Vec<Invitation>> {
  sqlx::query_as!(
    Invitation,
    r#"SELECT id AS "id: ids::InvitationId", account_id AS "account_id: AccountId",
              invited_by_user_id AS "invited_by_user_id: ids::UserId",
              accepted_by_user_id AS "accepted_by_user_id?: ids::UserId",
              email, role, code, token_hash,
              created_at, expires_at, accepted_at, revoked_at
       FROM invitations
       WHERE account_id = $1 AND accepted_at IS NULL AND revoked_at IS NULL AND expires_at > now()
       ORDER BY sid DESC"#,
    account_id.as_str(),
  )
  .fetch_all(exec)
  .await
}
