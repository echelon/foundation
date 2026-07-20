use sqlx::PgExecutor;

use super::Invitation;

/// Resolve an invitation from the short human code (e.g. `7F3K-QT9M`) typed on the join page.
pub async fn find_invitation_by_code<'e, E: PgExecutor<'e>>(
  exec: E,
  code: &str,
) -> sqlx::Result<Option<Invitation>> {
  sqlx::query_as!(
    Invitation,
    r#"SELECT id AS "id: ids::InvitationId", account_id AS "account_id: ids::AccountId",
              invited_by_user_id AS "invited_by_user_id: ids::UserId",
              accepted_by_user_id AS "accepted_by_user_id?: ids::UserId",
              email, role, code, token_hash,
              created_at, expires_at, accepted_at, revoked_at
       FROM invitations
       WHERE code = $1"#,
    code,
  )
  .fetch_optional(exec)
  .await
}
