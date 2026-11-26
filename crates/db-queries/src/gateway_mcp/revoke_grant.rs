use ids::{GatewayMcpGrantId, UserId};
use sqlx::PgExecutor;

pub async fn revoke_grant<'e, E: PgExecutor<'e>>(
    exec: E, grant: &GatewayMcpGrantId, user: &UserId, resource: &str,
) -> sqlx::Result<()> {
    sqlx::query!(
        r#"UPDATE gateway_mcp_grants SET revoked_at = now()
           WHERE id = $1 AND user_id = $2 AND resource = $3 AND revoked_at IS NULL"#,
        grant.as_str(), user.as_str(), resource,
    ).execute(exec).await?;
    Ok(())
}
