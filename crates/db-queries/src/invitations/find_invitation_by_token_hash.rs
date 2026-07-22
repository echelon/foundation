use sqlx::PgExecutor;

use super::Invitation;

/// Resolve an invitation from the magic-link token's sha256 (the caller hashes the plaintext).
pub async fn find_invitation_by_token_hash<'e, E: PgExecutor<'e>>(
  exec: E,
  token_hash: &str,
) -> sqlx::Result<Option<Invitation>> {
  sqlx::query_as!(
    Invitation,
    r#"SELECT id AS "id: ids::InvitationId", account_id AS "account_id: ids::AccountId",
              invited_by_user_id AS "invited_by_user_id: ids::UserId",
              accepted_by_user_id AS "accepted_by_user_id?: ids::UserId",
              email, role AS "role: enums::MemberRole", code, token_hash,
              created_at, expires_at, accepted_at, revoked_at
       FROM invitations
       WHERE token_hash = $1"#,
    token_hash,
  )
  .fetch_optional(exec)
  .await
}
