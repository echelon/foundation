use sqlx::PgPool;

use super::ClaimedProviderRun;

pub struct QueueReplacement<'a> {
    pub offer_id: &'a str,
    pub binding_digest: &'a str,
    pub request_payload: &'a [u8],
    pub estimated_usd_nanos: i64,
}

/// Requeue under the original lease after an authoritative pre-acceptance rejection.
pub async fn reschedule_queue_submission(
    pool: &PgPool,
    run: &ClaimedProviderRun,
    replacement: Option<QueueReplacement<'_>>,
    delay_ms: i32,
    max_retries: i32,
) -> sqlx::Result<bool> {
    let replacement = replacement.as_ref();
    let changed = sqlx::query!(
        "WITH ready AS (
             UPDATE gateway_requests r SET dispatched_at = NULL,
                 dispatch_after = statement_timestamp() + $4::integer * interval '1 millisecond',
                 estimated_usd_nanos = coalesce($9::bigint, r.estimated_usd_nanos)
             FROM gateway_provider_runs p
             WHERE r.id = $1 AND r.adapter = $3 AND r.adapter IN ('fal_queue', 'kinovi_queue', 'connector_queue')
               AND p.request_id = r.id AND p.lease_until = $2 AND p.lease_until > clock_timestamp()
               AND r.dispatched_at IS NOT NULL AND p.request_payload IS NOT NULL
               AND p.accepted_at IS NULL AND p.upstream_handle IS NULL
               AND NOT EXISTS (SELECT 1 FROM gateway_connector_completions c WHERE c.request_id = r.id)
               AND (r.provider_credential_version IS NULL OR coalesce($7::text, p.binding_digest) = p.binding_digest)
               AND num_nonnulls(r.failed_at, r.succeeded_at, r.canceled_at, r.uncertain_at) = 0
               AND jsonb_array_length(p.route_retries) < $5
               AND ($9::bigint IS NULL OR $9 <= coalesce(r.budget_reserved_usd_nanos, r.estimated_usd_nanos))
               AND statement_timestamp() + $4::integer * interval '1 millisecond'
                   < r.created_at + interval '15 minutes'
             RETURNING r.id, r.offer_id, r.estimated_usd_nanos
         )
         UPDATE gateway_provider_runs p SET lease_until = NULL,
             binding_digest = coalesce($7::text, p.binding_digest),
             request_payload = coalesce($8::bytea, p.request_payload),
             route_retries = p.route_retries || jsonb_build_object(
                 'at', statement_timestamp(),
                 'decision', jsonb_build_object(
                     'offer_id', coalesce(p.route_retries->-1->'decision'->>'next_offer_id', ready.offer_id),
                     'next_offer_id', coalesce($6::text, p.route_retries->-1->'decision'->>'next_offer_id', ready.offer_id),
                     'estimated_usd_nanos', ready.estimated_usd_nanos,
                     'delay_ms', $4::integer
                 )
             )
         FROM ready WHERE p.request_id = ready.id",
        run.request_id,
        run.lease,
        run.adapter,
        delay_ms,
        max_retries,
        replacement.map(|route| route.offer_id),
        replacement.map(|route| route.binding_digest),
        replacement.map(|route| route.request_payload),
        replacement.map(|route| route.estimated_usd_nanos),
    )
    .execute(pool)
    .await?
    .rows_affected();
    Ok(changed == 1)
}
