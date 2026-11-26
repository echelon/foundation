use ids::{AccountId, GatewayMcpGrantId, GatewayProjectId, UserId};
use sqlx::PgExecutor;

pub struct NewGrant<'a> {
    pub id: &'a GatewayMcpGrantId,
    pub user: &'a UserId,
    pub session_version: i32,
    pub account: &'a AccountId,
    pub project: &'a GatewayProjectId,
    pub client_ref: Option<&'a str>,
    pub client: &'a str,
    pub redirect_uri: &'a str,
    pub resource: &'a str,
    pub scopes: &'a [String],
    pub challenge: &'a str,
    pub code_hash: &'a str,
}

pub async fn create_grant<'e, E: PgExecutor<'e>>(exec: E, grant: NewGrant<'_>) -> sqlx::Result<()> {
    sqlx::query!(
        r#"INSERT INTO gateway_mcp_grants (
               id, user_id, session_version, account_id, project_id, client_ref,
               oauth_client_id, redirect_uri, resource, scopes, code_challenge, code_hash
           ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)"#,
        grant.id.as_str(), grant.user.as_str(), grant.session_version,
        grant.account.as_str(), grant.project.as_str(), grant.client_ref, grant.client,
        grant.redirect_uri, grant.resource, grant.scopes, grant.challenge, grant.code_hash,
    ).execute(exec).await?;
    Ok(())
}
