use ids::{AccountId, GatewayPrincipalId, GatewayProjectId};
use sqlx::PgExecutor;

pub struct NewPrincipal<'a> {
    pub id: &'a GatewayPrincipalId,
    pub account: &'a AccountId,
    pub project: &'a GatewayProjectId,
    pub name: &'a str,
    pub scopes: &'a [String],
    pub client_ref: Option<&'a str>,
}

pub async fn create_principal<'e, E: PgExecutor<'e>>(
    exec: E,
    principal: NewPrincipal<'_>,
) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO gateway_service_principals (id, account_id, project_id, name, scopes, client_ref)
         VALUES ($1, $2, $3, $4, $5, $6)",
        principal.id.as_str(), principal.account.as_str(), principal.project.as_str(),
        principal.name, principal.scopes, principal.client_ref,
    ).execute(exec).await?;
    Ok(())
}
