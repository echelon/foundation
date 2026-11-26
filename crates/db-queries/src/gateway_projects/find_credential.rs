use ids::{AccountId, GatewayPrincipalId, GatewayProjectId};
use sqlx::PgExecutor;

use super::CredentialScope;

pub async fn find_credential<'e, E: PgExecutor<'e>>(
    exec: E,
    hash: &str,
) -> sqlx::Result<Option<CredentialScope>> {
    sqlx::query_as!(
        CredentialScope,
        r#"SELECT p.account_id AS "account_id: AccountId", p.project_id AS "project_id: GatewayProjectId",
             p.id AS "principal_id: GatewayPrincipalId", p.scopes, p.client_ref
           FROM gateway_project_keys k JOIN gateway_service_principals p
             ON p.account_id = k.account_id AND p.project_id = k.project_id AND p.id = k.principal_id
           WHERE k.key_hash = $1 AND k.revoked_at IS NULL AND p.revoked_at IS NULL AND k.expires_at > now()"#,
        hash,
    ).fetch_optional(exec).await
}
