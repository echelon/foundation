use ids::{AccountId, GatewayPrincipalId, GatewayProjectId};
use sqlx::PgExecutor;

use super::PrincipalRow;

pub async fn list_principals<'e, E: PgExecutor<'e>>(
    exec: E,
    account: &AccountId,
    project: &GatewayProjectId,
) -> sqlx::Result<Vec<PrincipalRow>> {
    sqlx::query_as!(
        PrincipalRow,
        r#"SELECT id AS "id: GatewayPrincipalId", name, scopes, client_ref, revoked_at
           FROM gateway_service_principals WHERE account_id = $1 AND project_id = $2 ORDER BY created_at, id"#,
        account.as_str(), project.as_str(),
    ).fetch_all(exec).await
}
