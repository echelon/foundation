use ids::{SsoConnectionId, UserId};
use sqlx::PgExecutor;

use super::SsoConnection;

#[derive(Debug, Clone)]
pub struct NewSsoConnection<'a> {
  pub user_id: &'a UserId,
  pub provider: &'a str,
  pub provider_user_id: &'a str,
  pub provider_email: &'a str,
  pub provider_user_display_name: Option<&'a str>,
}

/// Link a provider identity to a user. `(provider, provider_user_id)` and `(user_id, provider)`
/// are both UNIQUE — a duplicate link is a DB error, surfaced to the caller.
pub async fn create_sso_connection<'e, E: PgExecutor<'e>>(
  exec: E,
  new: NewSsoConnection<'_>,
) -> sqlx::Result<SsoConnection> {
  let id = SsoConnectionId::generate();
  sqlx::query_as!(
    SsoConnection,
    r#"INSERT INTO user_sso_connections
         (id, user_id, provider, provider_user_id, provider_email,
          provider_user_display_name, last_used_at)
       VALUES ($1, $2, $3, $4, $5, $6, now())
       RETURNING id AS "id: SsoConnectionId", user_id AS "user_id: UserId",
                 provider, provider_user_id, provider_email,
                 provider_user_display_name, created_at, last_used_at"#,
    id.as_str(),
    new.user_id.as_str(),
    new.provider,
    new.provider_user_id,
    new.provider_email,
    new.provider_user_display_name,
  )
  .fetch_one(exec)
  .await
}
