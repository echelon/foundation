use ids::{AccountId, GatewayProjectId};
use sqlx::PgExecutor;

use super::PolicyRow;

pub async fn current<'e>(
    exec: impl PgExecutor<'e>,
    account: &AccountId,
    project: Option<&GatewayProjectId>,
) -> sqlx::Result<Vec<PolicyRow>> {
    sqlx::query_as!(
        PolicyRow,
        r#"(SELECT revision AS "revision!", policy AS "policy!", project_id FROM gateway_routing_policies
          WHERE account_id = $1 AND project_id IS NULL ORDER BY revision DESC LIMIT 1)
         UNION ALL
         (SELECT revision, policy, project_id FROM gateway_routing_policies
          WHERE account_id = $1 AND project_id = $2 ORDER BY revision DESC LIMIT 1)"#,
        account.as_str(),
        project.map(GatewayProjectId::as_str),
    )
    .fetch_all(exec)
    .await
}
