use ids::{AccountId, GatewayPrincipalId, GatewayProjectId, GatewayProjectKeyId};
use sqlx::PgExecutor;

use super::KeyRow;

pub async fn list_keys<'e, E: PgExecutor<'e>>(
    exec: E,
    account: &AccountId,
    project: &GatewayProjectId,
) -> sqlx::Result<Vec<KeyRow>> {
    sqlx::query_as!(
        KeyRow,
        r#"SELECT id AS "id: GatewayProjectKeyId", principal_id AS "principal_id: GatewayPrincipalId",
                  key_prefix, expires_at, revoked_at
           FROM gateway_project_keys WHERE account_id = $1 AND project_id = $2 ORDER BY created_at, id"#,
        account.as_str(), project.as_str(),
    ).fetch_all(exec).await
}
