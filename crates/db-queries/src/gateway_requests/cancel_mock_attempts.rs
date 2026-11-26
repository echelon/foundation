use ids::GatewayRequestId;
use sqlx::PgExecutor;

use super::RequestScope;

/// Run after acquiring the request lock in the same transaction as cancellation.
pub async fn cancel_mock_attempts<'e, E: PgExecutor<'e>>(
    exec: E,
    scope: RequestScope<'_>,
    request: &GatewayRequestId,
) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE gateway_provider_attempts a SET canceled_at = r.canceled_at
         FROM gateway_requests r WHERE r.account_id = $1 AND r.project_id = $2 AND ($3::text IS NULL OR r.client_ref = $3) AND r.id = $4
           AND a.account_id = r.account_id AND a.request_id = r.id
           AND r.canceled_at IS NOT NULL AND a.completed_at IS NULL AND a.canceled_at IS NULL",
        scope.account.as_str(), scope.project.as_str(), scope.client_ref,
        request.as_str(),
    )
    .execute(exec)
    .await?;
    Ok(())
}
