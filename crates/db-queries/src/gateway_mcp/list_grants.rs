use ids::{GatewayMcpGrantId, GatewayProjectId, UserId};
use sqlx::PgExecutor;

use super::Connection;

pub async fn list_grants<'e, E: PgExecutor<'e>>(
    exec: E, user: &UserId, resource: &str,
) -> sqlx::Result<Vec<Connection>> {
    sqlx::query_as!(
        Connection,
        r#"SELECT id AS "id: GatewayMcpGrantId", project_id AS "project_id: GatewayProjectId",
                  oauth_client_id, scopes
           FROM gateway_mcp_grants
           WHERE user_id = $1 AND resource = $2 AND revoked_at IS NULL AND expires_at > now()
             AND (access_hash IS NOT NULL OR code_expires_at > now())
           ORDER BY created_at DESC"#,
        user.as_str(), resource,
    ).fetch_all(exec).await
}
