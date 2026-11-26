use sqlx::PgExecutor;

use super::TokenRotation;

pub struct CodeExchange<'a> {
    pub rotation: TokenRotation<'a>,
    pub redirect_uri: &'a str,
    pub challenge: &'a str,
}

pub async fn exchange_code<'e, E: PgExecutor<'e>>(
    exec: E, exchange: CodeExchange<'_>,
) -> sqlx::Result<Option<Vec<String>>> {
    let token = exchange.rotation;
    sqlx::query_scalar!(
        r#"UPDATE gateway_mcp_grants g
           SET code_hash = NULL, access_hash = $2, refresh_hash = $3,
               access_expires_at = LEAST(expires_at, now() + INTERVAL '15 minutes')
           WHERE code_hash = $1 AND oauth_client_id = $4 AND resource = $5
             AND redirect_uri = $6 AND code_challenge = $7 AND code_expires_at > now()
             AND revoked_at IS NULL AND expires_at > now()
             AND EXISTS (SELECT 1 FROM users u WHERE u.id = g.user_id
                         AND u.session_version = g.session_version AND u.deleted_at IS NULL)
           RETURNING scopes"#,
        token.previous_hash, token.access_hash, token.refresh_hash, token.client,
        token.resource, exchange.redirect_uri, exchange.challenge,
    ).fetch_optional(exec).await
}
