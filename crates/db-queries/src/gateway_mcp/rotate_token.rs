use sqlx::PgExecutor;

use super::TokenRotation;

pub async fn rotate_token<'e, E: PgExecutor<'e>>(
    exec: E, token: TokenRotation<'_>,
) -> sqlx::Result<Option<Vec<String>>> {
    sqlx::query_scalar!(
        r#"UPDATE gateway_mcp_grants g
           SET access_hash = $2, refresh_hash = $3,
               access_expires_at = LEAST(expires_at, now() + INTERVAL '15 minutes')
           WHERE refresh_hash = $1 AND oauth_client_id = $4 AND resource = $5
             AND revoked_at IS NULL AND expires_at > now()
             AND EXISTS (SELECT 1 FROM users u WHERE u.id = g.user_id
                         AND u.session_version = g.session_version AND u.deleted_at IS NULL)
           RETURNING scopes"#,
        token.previous_hash, token.access_hash, token.refresh_hash, token.client, token.resource,
    ).fetch_optional(exec).await
}
