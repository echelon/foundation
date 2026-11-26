use ids::GatewayRequestId;
use sqlx::PgExecutor;

pub async fn read_route_retries<'e, E: PgExecutor<'e>>(
    exec: E,
    request: &GatewayRequestId,
) -> sqlx::Result<Option<serde_json::Value>> {
    sqlx::query_scalar!(
        "SELECT route_retries FROM gateway_provider_runs WHERE request_id = $1",
        request.as_str(),
    )
    .fetch_optional(exec)
    .await
}
