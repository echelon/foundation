use ids::GatewayRequestId;
use sqlx::PgExecutor;

use super::RequestScope;

pub async fn cancel_request<'e, E: PgExecutor<'e>>(
    exec: E,
    scope: RequestScope<'_>,
    request: &GatewayRequestId,
) -> sqlx::Result<bool> {
    let changed = sqlx::query(
        "UPDATE gateway_requests r SET canceled_at = clock_timestamp()
         WHERE r.account_id = $1 AND r.project_id = $2 AND ($3::text IS NULL OR r.client_ref = $3) AND r.id = $4
           AND gateway_request_can_cancel(r)",
    )
    .bind(scope.account.as_str())
    .bind(scope.project.as_str())
    .bind(scope.client_ref)
    .bind(request.as_str())
    .execute(exec)
    .await?;
    Ok(changed.rows_affected() == 1)
}

// This update and cancellation serialize on the request row. Leave the boundary
// set after a retry because the previous paid call may have reached the provider.
pub async fn begin_provider_submission(
    pool: &sqlx::PgPool,
    request_id: &str,
    lease: chrono::DateTime<chrono::Utc>,
) -> sqlx::Result<bool> {
    let saved = sqlx::query(
        "UPDATE gateway_requests r
         SET billable_dispatch_at = CASE WHEN r.estimated_usd_nanos > 0
             THEN coalesce(r.billable_dispatch_at, clock_timestamp()) ELSE r.billable_dispatch_at END,
             connector_submit_started_at = CASE WHEN r.adapter = 'connector_queue'
                 THEN clock_timestamp() ELSE r.connector_submit_started_at END
         FROM gateway_provider_runs run
         WHERE r.id = $1 AND run.request_id = r.id AND run.lease_until = $2
           AND run.lease_until > clock_timestamp()
           AND num_nonnulls(r.succeeded_at, r.canceled_at, r.failed_at, r.uncertain_at) = 0",
    )
    .bind(request_id)
    .bind(lease)
    .execute(pool)
    .await?;
    Ok(saved.rows_affected() == 1)
}
