use sqlx::PgExecutor;

use super::SsoConnection;

/// Resolve a login by the provider's stable subject id (`sub` / numeric id). This — not the
/// email — is the identity anchor.
pub async fn find_sso_by_provider_sub<'e, E: PgExecutor<'e>>(
  exec: E,
  provider: &str,
  provider_user_id: &str,
) -> sqlx::Result<Option<SsoConnection>> {
  sqlx::query_as!(
    SsoConnection,
    r#"SELECT id AS "id: ids::SsoConnectionId", user_id AS "user_id: ids::UserId",
              provider, provider_user_id, provider_email,
              provider_user_display_name, created_at, last_used_at
       FROM user_sso_connections
       WHERE provider = $1 AND provider_user_id = $2"#,
    provider,
    provider_user_id,
  )
  .fetch_optional(exec)
  .await
}
