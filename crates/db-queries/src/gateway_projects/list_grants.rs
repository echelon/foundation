use ids::{AccountId, GatewayProjectId, UserId};
use sqlx::PgExecutor;

use super::GrantRow;

pub async fn list_grants<'e, E: PgExecutor<'e>>(
    exec: E,
    account: &AccountId,
    project: &GatewayProjectId,
) -> sqlx::Result<Vec<GrantRow>> {
    sqlx::query_as!(
        GrantRow,
        r#"SELECT m.user_id AS "user_id: UserId", g.role, g.client_ref
           FROM gateway_project_grants g JOIN account_members m ON m.account_id = g.account_id AND m.id = g.membership_id
           WHERE g.account_id = $1 AND g.project_id = $2 ORDER BY m.user_id"#,
        account.as_str(), project.as_str(),
    ).fetch_all(exec).await
}
