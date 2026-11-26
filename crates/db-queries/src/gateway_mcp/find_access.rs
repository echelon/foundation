use ids::{AccountId, GatewayMcpGrantId, GatewayProjectId, UserId};
use sqlx::PgExecutor;

use super::Grant;

pub async fn find_access<'e, E: PgExecutor<'e>>(
    exec: E, hash: &str, resource: &str,
) -> sqlx::Result<Option<Grant>> {
    sqlx::query_as!(
        Grant,
        r#"SELECT g.id AS "id: GatewayMcpGrantId", g.user_id AS "user_id: UserId",
                  g.account_id AS "account_id: AccountId", g.project_id AS "project_id: GatewayProjectId",
                  g.client_ref, g.oauth_client_id, g.scopes
           FROM gateway_mcp_grants g
           JOIN users u ON u.id = g.user_id AND u.session_version = g.session_version
           WHERE g.access_hash = $1 AND g.resource = $2 AND g.access_expires_at > now()
             AND g.revoked_at IS NULL AND u.deleted_at IS NULL"#,
        hash, resource,
    ).fetch_optional(exec).await
}
