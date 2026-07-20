use ids::UserId;
use sqlx::PgExecutor;

use super::SsoConnection;

/// The connected providers shown on the account-security page, newest first.
pub async fn list_sso_for_user<'e, E: PgExecutor<'e>>(
  exec: E,
  user_id: &UserId,
) -> sqlx::Result<Vec<SsoConnection>> {
  sqlx::query_as!(
    SsoConnection,
    r#"SELECT id AS "id: ids::SsoConnectionId", user_id AS "user_id: UserId",
              provider, provider_user_id, provider_email,
              provider_user_display_name, created_at, last_used_at
       FROM user_sso_connections
       WHERE user_id = $1
       ORDER BY sid DESC"#,
    user_id.as_str(),
  )
  .fetch_all(exec)
  .await
}
