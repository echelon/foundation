use ids::{AccountId, GatewayPrincipalId, GatewayProjectId};
use sqlx::{PgConnection, PgExecutor};

pub async fn attached_principal<'e>(
    exec: impl PgExecutor<'e>,
    account: &AccountId,
    project: &GatewayProjectId,
) -> sqlx::Result<Option<GatewayPrincipalId>> {
    sqlx::query_scalar(
        "SELECT attached_principal_id FROM gateway_projects WHERE account_id = $1 AND id = $2",
    )
    .bind(account.as_str())
    .bind(project.as_str())
    .fetch_one(exec)
    .await
}

pub async fn lock_project_key(
    conn: &mut PgConnection,
    account: &AccountId,
    project: &GatewayProjectId,
) -> sqlx::Result<Option<GatewayPrincipalId>> {
    sqlx::query_scalar(
        "SELECT attached_principal_id FROM gateway_projects WHERE account_id = $1 AND id = $2 FOR UPDATE",
    )
    .bind(account.as_str())
    .bind(project.as_str())
    .fetch_one(conn)
    .await
}

pub async fn attach_principal(
    conn: &mut PgConnection,
    account: &AccountId,
    project: &GatewayProjectId,
    principal: &GatewayPrincipalId,
) -> sqlx::Result<bool> {
    let changed = sqlx::query(
        "UPDATE gateway_projects p SET attached_principal_id = $3
         WHERE p.account_id = $1 AND p.id = $2
           AND EXISTS (
             SELECT 1 FROM gateway_service_principals principal
             JOIN gateway_project_keys key
               ON (key.account_id, key.project_id, key.principal_id) =
                  (principal.account_id, principal.project_id, principal.id)
             WHERE (principal.account_id, principal.project_id, principal.id) = ($1, $2, $3)
               AND principal.revoked_at IS NULL AND key.revoked_at IS NULL AND key.expires_at > now()
           )",
    )
    .bind(account.as_str())
    .bind(project.as_str())
    .bind(principal.as_str())
    .execute(conn)
    .await?;
    Ok(changed.rows_affected() == 1)
}

pub async fn detach_principal(
    conn: &mut PgConnection,
    account: &AccountId,
    project: &GatewayProjectId,
    principal: &GatewayPrincipalId,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE gateway_projects SET attached_principal_id = NULL
         WHERE (account_id, id, attached_principal_id) = ($1, $2, $3)",
    )
    .bind(account.as_str())
    .bind(project.as_str())
    .bind(principal.as_str())
    .execute(conn)
    .await?;
    Ok(())
}
