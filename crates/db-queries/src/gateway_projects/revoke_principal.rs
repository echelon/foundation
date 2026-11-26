use ids::{AccountId, GatewayPrincipalId, GatewayProjectId};
use sqlx::PgExecutor;

pub async fn revoke_principal<'e, E: PgExecutor<'e>>(
    exec: E,
    account: &AccountId,
    project: &GatewayProjectId,
    principal: &GatewayPrincipalId,
) -> sqlx::Result<bool> {
    let rows = sqlx::query!(
        "UPDATE gateway_service_principals SET revoked_at = now()
         WHERE account_id = $1 AND project_id = $2 AND id = $3 AND revoked_at IS NULL",
        account.as_str(),
        project.as_str(),
        principal.as_str(),
    )
    .execute(exec)
    .await?
    .rows_affected();
    Ok(rows == 1)
}
