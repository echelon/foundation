use ids::InvitationId;
use sqlx::PgExecutor;

use super::Invitation;

pub async fn find_invitation_by_id<'e, E: PgExecutor<'e>>(
  exec: E,
  id: &InvitationId,
) -> sqlx::Result<Option<Invitation>> {
  sqlx::query_as!(
    Invitation,
    r#"SELECT id AS "id: InvitationId", account_id AS "account_id: ids::AccountId",
              invited_by_user_id AS "invited_by_user_id: ids::UserId",
              accepted_by_user_id AS "accepted_by_user_id?: ids::UserId",
              email, role AS "role: enums::MemberRole", code, token_hash,
              created_at, expires_at, accepted_at, revoked_at
       FROM invitations
       WHERE id = $1"#,
    id.as_str(),
  )
  .fetch_optional(exec)
  .await
}
