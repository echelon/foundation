use ids::UserId;
use sqlx::PgExecutor;

use super::Session;

/// Active sessions for the account-settings "devices" list (auth.md §3): not revoked, not expired,
/// newest first. Staff impersonation sessions are filtered out — they never show to the customer.
pub async fn list_live_sessions_for_user<'e, E: PgExecutor<'e>>(
  exec: E,
  user_id: &UserId,
) -> sqlx::Result<Vec<Session>> {
  sqlx::query_as!(
    Session,
    r#"SELECT id AS "id: ids::SessionId", user_id AS "user_id: UserId",
              impersonator_user_id AS "impersonator_user_id?: UserId",
              host(ip) AS "ip?: String", user_agent, device,
              created_via AS "created_via: enums::SessionCreationType",
              created_at, expires_at, revoked_at
       FROM sessions
       WHERE user_id = $1 AND revoked_at IS NULL AND expires_at > now()
             AND impersonator_user_id IS NULL
       ORDER BY sid DESC"#,
    user_id.as_str(),
  )
  .fetch_all(exec)
  .await
}
