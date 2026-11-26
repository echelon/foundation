use chrono::{DateTime, Utc};
use ids::{AccountId, GatewayPrincipalId, GatewayProjectId, GatewayProjectKeyId};
use sqlx::PgExecutor;

pub struct NewProjectKey<'a> {
    pub id: &'a GatewayProjectKeyId,
    pub account: &'a AccountId,
    pub project: &'a GatewayProjectId,
    pub principal: &'a GatewayPrincipalId,
    pub prefix: &'a str,
    pub hash: &'a str,
    pub expires_at: DateTime<Utc>,
}

pub async fn insert_key<'e, E: PgExecutor<'e>>(
    exec: E,
    key: NewProjectKey<'_>,
) -> sqlx::Result<bool> {
    let rows = sqlx::query!(
        "INSERT INTO gateway_project_keys (id, account_id, project_id, principal_id, key_prefix, key_hash, expires_at)
         SELECT $1, account_id, project_id, id, $5, $6, $7 FROM gateway_service_principals
         WHERE account_id = $2 AND project_id = $3 AND id = $4 AND revoked_at IS NULL FOR UPDATE",
        key.id.as_str(), key.account.as_str(), key.project.as_str(), key.principal.as_str(),
        key.prefix, key.hash, key.expires_at,
    ).execute(exec).await?.rows_affected();
    Ok(rows == 1)
}
