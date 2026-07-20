use ids::UserId;
use sqlx::PgExecutor;

use super::Session;

/// Active sessions for the account-settings "devices" list: not revoked, not expired, newest first.
pub async fn list_live_sessions_for_user<'e, E: PgExecutor<'e>>(
  exec: E,
  user_id: &UserId,
) -> sqlx::Result<Vec<Session>> {
  sqlx::query_as!(
    Session,
    r#"SELECT id AS "id: ids::SessionId", user_id AS "user_id: UserId",
              impersonator_user_id AS "impersonator_user_id?: UserId",
              created_at, expires_at, revoked_at
       FROM sessions
       WHERE user_id = $1 AND revoked_at IS NULL AND expires_at > now()
       ORDER BY sid DESC"#,
    user_id.as_str(),
  )
  .fetch_all(exec)
  .await
}
