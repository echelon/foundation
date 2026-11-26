use ids::{AccountId, GatewayProjectId};
use sqlx::PgExecutor;

pub async fn create_project<'e, E: PgExecutor<'e>>(
    exec: E,
    account: &AccountId,
    project: &GatewayProjectId,
    name: &str,
) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO gateway_projects (id, account_id, name) VALUES ($1, $2, $3)",
        project.as_str(),
        account.as_str(),
        name,
    )
    .execute(exec)
    .await?;
    Ok(())
}
