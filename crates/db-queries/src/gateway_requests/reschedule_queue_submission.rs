use sqlx::PgPool;

use super::{connector_completions, ClaimedProviderRun};

/// Requeue under the original lease after an authoritative pre-acceptance rejection.
pub async fn reschedule_queue_submission(
    pool: &PgPool,
    run: &ClaimedProviderRun,
    delay_ms: i32,
    max_retries: i32,
) -> sqlx::Result<bool> {
    // Read receipts after acquiring the lock, even when a callback was still
    // uncommitted at the time this worker received the provider response.
    let mut tx = pool.begin_with("BEGIN ISOLATION LEVEL READ COMMITTED").await?;
    if !connector_completions::lock_request(&mut tx, &run.request_id).await? {
        return Ok(false);
    }
    let changed = sqlx::query!(
        "WITH ready AS (
             UPDATE gateway_requests r SET dispatched_at = NULL,
                 dispatch_after = statement_timestamp() + $3::integer * interval '1 millisecond'
             FROM gateway_provider_runs p
             WHERE r.id = $1 AND r.adapter = 'connector_queue'
               AND p.request_id = r.id AND p.lease_until = $2 AND p.lease_until > clock_timestamp()
               AND r.dispatched_at IS NOT NULL AND p.request_payload IS NOT NULL
               AND p.accepted_at IS NULL AND p.upstream_handle IS NULL
               AND NOT EXISTS (
                   SELECT 1 FROM gateway_connector_completions c WHERE c.request_id = r.id AND c.received_at IS NOT NULL
               )
               AND num_nonnulls(r.failed_at, r.succeeded_at, r.canceled_at, r.uncertain_at) = 0
               AND jsonb_array_length(p.route_retries) < $4
               AND statement_timestamp() + $3::integer * interval '1 millisecond'
                   < r.created_at + interval '15 minutes'
             RETURNING r.id, r.offer_id, r.estimated_usd_nanos
         )
         UPDATE gateway_provider_runs p SET lease_until = NULL,
             route_retries = p.route_retries || jsonb_build_object(
                 'at', statement_timestamp(),
                 'decision', jsonb_build_object(
                     'offer_id', coalesce(p.route_retries->-1->'decision'->>'next_offer_id', ready.offer_id),
                     'next_offer_id', coalesce(p.route_retries->-1->'decision'->>'next_offer_id', ready.offer_id),
                     'estimated_usd_nanos', ready.estimated_usd_nanos,
                     'delay_ms', $3::integer
                 )
             )
         FROM ready WHERE p.request_id = ready.id",
        run.request_id,
        run.lease,
        delay_ms,
        max_retries,
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    tx.commit().await?;
    Ok(changed == 1)
}
