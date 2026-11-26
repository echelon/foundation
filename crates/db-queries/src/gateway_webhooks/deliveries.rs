use chrono::{DateTime, Utc};
use ids::GatewayWebhookDeliveryId;
use serde::Serialize;
use sqlx::PgExecutor;

use crate::gateway_requests::RequestScope;

#[derive(Serialize)]
pub struct Delivery {
    pub id: String,
    pub request_id: String,
    pub subscription_id: String,
    pub event_type: String,
    pub state: String,
    pub occurred_at: DateTime<Utc>,
    pub available_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub attempt_count: i32,
    pub last_error: Option<String>,
}

pub async fn deliveries<'e, E: PgExecutor<'e>>(
    exec: E,
    scope: RequestScope<'_>,
    id: Option<&GatewayWebhookDeliveryId>,
    before: Option<&GatewayWebhookDeliveryId>,
    limit: i64,
) -> sqlx::Result<Vec<Delivery>> {
    sqlx::query_as!(
        Delivery,
        "SELECT d.id, d.request_id, r.webhook_subscription_id AS \"subscription_id!\", d.event_type, d.state,
           d.occurred_at, d.available_at, d.finished_at, d.attempt_count, d.last_error
         FROM gateway_webhook_deliveries d JOIN gateway_requests r ON r.id = d.request_id
         WHERE r.account_id = $1 AND r.project_id = $2 AND ($3::text IS NULL OR d.id = $3)
           AND ($4::text IS NULL OR d.id < $4)
         ORDER BY d.id DESC LIMIT $5",
        scope.account.as_str(), scope.project.as_str(), id.map(GatewayWebhookDeliveryId::as_str),
        before.map(GatewayWebhookDeliveryId::as_str), limit,
    )
    .fetch_all(exec)
    .await
}

#[derive(Serialize)]
pub struct DeliveryAttempt {
    pub number: i32,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub outcome: Option<String>,
    pub http_status: Option<i16>,
}

pub async fn attempts<'e, E: PgExecutor<'e>>(
    exec: E,
    scope: RequestScope<'_>,
    delivery: &GatewayWebhookDeliveryId,
) -> sqlx::Result<Vec<DeliveryAttempt>> {
    sqlx::query_as!(
        DeliveryAttempt,
        "SELECT a.number, a.started_at, a.finished_at, a.outcome, a.http_status
         FROM gateway_webhook_attempts a JOIN gateway_webhook_deliveries d ON d.id = a.delivery_id
           JOIN gateway_requests r ON r.id = d.request_id
         WHERE r.account_id = $1 AND r.project_id = $2 AND a.delivery_id = $3
         ORDER BY a.number DESC LIMIT 50",
        scope.account.as_str(),
        scope.project.as_str(),
        delivery.as_str(),
    )
    .fetch_all(exec)
    .await
}

pub async fn redrive<'e, E: PgExecutor<'e>>(
    exec: E,
    scope: RequestScope<'_>,
    delivery: &GatewayWebhookDeliveryId,
) -> sqlx::Result<bool> {
    let res = sqlx::query!(
        "UPDATE gateway_webhook_deliveries d
         SET state = 'pending', attempt_baseline = attempt_count, available_at = clock_timestamp(),
             deadline_at = clock_timestamp() + interval '15 minutes', finished_at = NULL, last_error = NULL
         FROM gateway_requests r JOIN gateway_webhook_subscriptions s ON s.id = r.webhook_subscription_id
         WHERE d.request_id = r.id AND r.account_id = $1 AND r.project_id = $2 AND d.id = $3
           AND d.state = 'dead' AND s.revoked_at IS NULL",
        scope.account.as_str(), scope.project.as_str(), delivery.as_str(),
    )
    .execute(exec)
    .await?;
    Ok(res.rows_affected() == 1)
}
