use ids::GatewayRequestId;
use sqlx::PgExecutor;

use super::super::RequestScope;

pub async fn read_compute_attempts<'e, E: PgExecutor<'e>>(
    exec: E,
    scope: RequestScope<'_>,
    id: &GatewayRequestId,
) -> sqlx::Result<Option<serde_json::Value>> {
    sqlx::query_scalar!(
        r#"SELECT p.compute_attempts
           FROM gateway_provider_runs p JOIN gateway_requests r ON r.id = p.request_id
           WHERE r.id = $1 AND r.account_id = $2 AND r.project_id = $3
             AND ($4::text IS NULL OR r.client_ref = $4)"#,
        id.as_str(),
        scope.account.as_str(),
        scope.project.as_str(),
        scope.client_ref,
    )
    .fetch_optional(exec)
    .await
}
