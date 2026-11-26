use ids::{AccountId, GatewayProjectId, UserId};
use sqlx::PgExecutor;

pub async fn insert<'e>(
    exec: impl PgExecutor<'e>,
    account: &AccountId,
    project: Option<&GatewayProjectId>,
    revision: i32,
    policy: &serde_json::Value,
    actor: &UserId,
) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO gateway_routing_policies (account_id, project_id, revision, policy, created_by)
         VALUES ($1, $2, $3, $4, $5)",
        account.as_str(),
        project.map(GatewayProjectId::as_str),
        revision,
        policy,
        actor.as_str(),
    )
    .execute(exec)
    .await?;
    Ok(())
}
