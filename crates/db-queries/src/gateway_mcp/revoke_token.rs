use sqlx::PgExecutor;

pub struct Revocation<'a> {
    pub hash: &'a str,
    pub client: &'a str,
    pub resource: &'a str,
}

pub async fn revoke_token<'e, E: PgExecutor<'e>>(
    exec: E, token: Revocation<'_>,
) -> sqlx::Result<()> {
    sqlx::query!(
        r#"UPDATE gateway_mcp_grants SET revoked_at = now()
           WHERE (access_hash = $1 OR refresh_hash = $1) AND oauth_client_id = $2
             AND resource = $3 AND revoked_at IS NULL"#,
        token.hash, token.client, token.resource,
    ).execute(exec).await?;
    Ok(())
}
