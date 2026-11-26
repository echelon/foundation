use ids::{AccountId, GatewayProjectId, GatewayProjectKeyId};
use sqlx::PgExecutor;

pub async fn revoke_key<'e, E: PgExecutor<'e>>(
    exec: E,
    account: &AccountId,
    project: &GatewayProjectId,
    key: &GatewayProjectKeyId,
) -> sqlx::Result<bool> {
    let rows = sqlx::query!(
        "UPDATE gateway_project_keys SET revoked_at = now()
         WHERE account_id = $1 AND project_id = $2 AND id = $3 AND revoked_at IS NULL",
        account.as_str(),
        project.as_str(),
        key.as_str(),
    )
    .execute(exec)
    .await?
    .rows_affected();
    Ok(rows == 1)
}
