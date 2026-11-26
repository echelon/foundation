use chrono::{DateTime, Utc};
use ids::GatewayRequestId;
use sqlx::PgPool;

/// Accept only the first hint and keep an acknowledged poll's lease and deadline.
pub async fn receive(pool: &PgPool, request: &GatewayRequestId) -> sqlx::Result<()> {
    sqlx::query(
        "WITH eligible AS (
             SELECT r.id, p.upstream_handle IS NULL AS needs_lookup
             FROM gateway_requests r JOIN gateway_provider_runs p ON p.request_id = r.id
             WHERE r.id = $1 AND r.adapter = 'connector_queue' AND r.dispatched_at IS NOT NULL
               AND num_nonnulls(r.succeeded_at, r.failed_at, r.canceled_at) = 0
               AND r.content_removed_at IS NULL
               AND (p.poll_deadline > clock_timestamp() OR
                   (p.upstream_handle IS NULL AND r.dispatched_at > clock_timestamp() - interval '15 minutes'))
             FOR UPDATE OF r
         ), received AS (
             INSERT INTO gateway_connector_completions (request_id, next_lookup_at)
             SELECT id, CASE WHEN needs_lookup THEN clock_timestamp() END FROM eligible
             ON CONFLICT (request_id) DO NOTHING RETURNING request_id
         )
         UPDATE gateway_provider_runs p SET poll_after = least(p.poll_after, clock_timestamp())
         FROM received WHERE p.request_id = received.request_id AND p.poll_after IS NOT NULL",
    )
    .bind(request.as_str())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn received(pool: &PgPool, request: &str) -> sqlx::Result<bool> {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM gateway_connector_completions WHERE request_id = $1)")
        .bind(request)
        .fetch_one(pool)
        .await
}

#[derive(sqlx::FromRow)]
pub struct Lookup {
    pub request_id: String,
    pub binding_digest: String,
    pub lease_until: DateTime<Utc>,
    pub trace_id: Option<String>,
    pub admission_span_id: Option<String>,
}

pub async fn claim_lookup(pool: &PgPool, bindings: &[String]) -> sqlx::Result<Option<Lookup>> {
    sqlx::query_as(
        "WITH eligible AS (
             SELECT c.request_id, p.binding_digest, r.trace_id, r.admission_span_id
             FROM gateway_connector_completions c
             JOIN gateway_provider_runs p ON p.request_id = c.request_id
             JOIN gateway_requests r ON r.id = c.request_id
             WHERE c.next_lookup_at BETWEEN statement_timestamp() - interval '15 minutes' AND statement_timestamp()
               AND p.binding_digest = ANY($1) AND p.upstream_handle IS NULL
               AND p.error_code IN ('connector_submission_unconfirmed', 'submission_interrupted')
               AND r.uncertain_at IS NOT NULL AND r.dispatched_at > statement_timestamp() - interval '15 minutes'
               AND num_nonnulls(r.succeeded_at, r.failed_at, r.canceled_at, r.content_removed_at) = 0
             ORDER BY c.next_lookup_at, c.request_id LIMIT 1 FOR UPDATE OF r SKIP LOCKED
         )
         UPDATE gateway_connector_completions c SET next_lookup_at = clock_timestamp() + interval '10 seconds'
         FROM eligible WHERE c.request_id = eligible.request_id
         RETURNING c.request_id, eligible.binding_digest, c.next_lookup_at AS lease_until,
                   eligible.trace_id, eligible.admission_span_id",
    )
    .bind(bindings)
    .fetch_optional(pool)
    .await
}

/// The worker resolved the handle at the original connector, using the request ID.
pub async fn reconcile(pool: &PgPool, lookup: &Lookup, handle: &str) -> sqlx::Result<()> {
    sqlx::query(
        "WITH eligible AS (
             SELECT r.id, r.uncertain_at FROM gateway_requests r
             JOIN gateway_provider_runs p ON p.request_id = r.id
             JOIN gateway_connector_completions c ON c.request_id = r.id
             WHERE r.id = $1 AND p.binding_digest = $2 AND p.upstream_handle IS NULL
               AND p.error_code IN ('connector_submission_unconfirmed', 'submission_interrupted')
               AND c.next_lookup_at = $3 AND c.next_lookup_at > clock_timestamp()
               AND r.uncertain_at IS NOT NULL AND r.dispatched_at > clock_timestamp() - interval '15 minutes'
               AND num_nonnulls(r.succeeded_at, r.failed_at, r.canceled_at, r.content_removed_at) = 0
               AND NOT EXISTS (SELECT 1 FROM gateway_provider_runs t WHERE t.binding_digest = $2
                   AND t.upstream_handle = $4 AND t.accepted_at IS NOT NULL)
             FOR UPDATE OF r
         ), acknowledged AS (
             UPDATE gateway_provider_runs p SET upstream_handle = $4, accepted_at = clock_timestamp(),
                 request_payload = NULL, lease_until = NULL, error_code = NULL, queue_state = 'in_queue',
                 poll_after = clock_timestamp(), poll_deadline = clock_timestamp() + interval '15 minutes'
             FROM eligible WHERE p.request_id = eligible.id RETURNING p.request_id
         ), recovered AS (
             UPDATE gateway_requests r SET uncertain_at = NULL FROM acknowledged
             WHERE r.id = acknowledged.request_id RETURNING r.id
         )
         UPDATE gateway_connector_completions c SET next_lookup_at = NULL,
             recovered_from = eligible.uncertain_at, reconciled_at = clock_timestamp()
         FROM recovered JOIN eligible ON eligible.id = recovered.id WHERE c.request_id = recovered.id",
    )
    .bind(&lookup.request_id)
    .bind(&lookup.binding_digest)
    .bind(lookup.lease_until)
    .bind(handle)
    .execute(pool)
    .await?;
    Ok(())
}
