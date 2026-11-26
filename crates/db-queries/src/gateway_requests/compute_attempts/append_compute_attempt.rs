use sqlx::PgExecutor;

use super::ComputeAttemptLease;

pub async fn append_compute_attempt<'e, E: PgExecutor<'e>>(
    exec: E,
    lease: ComputeAttemptLease<'_>,
    selection: &serde_json::Value,
) -> sqlx::Result<Option<i32>> {
    sqlx::query_scalar!(
        r#"UPDATE gateway_provider_runs p
           SET compute_attempts = p.compute_attempts || jsonb_build_array(
               jsonb_build_object('selection', $3::jsonb, 'started_at', clock_timestamp())
           )
           WHERE p.request_id = $1 AND p.lease_until = $2 AND p.lease_until > clock_timestamp()
             AND jsonb_array_length(p.compute_attempts) < 4
             AND EXISTS (SELECT 1 FROM gateway_requests r
                         WHERE r.id = p.request_id AND r.adapter = 'openai_compatible_chat'
                           AND num_nonnulls(r.succeeded_at, r.failed_at, r.canceled_at, r.uncertain_at) = 0)
           RETURNING jsonb_array_length(p.compute_attempts) AS "slot!""#,
        lease.request.as_str(),
        lease.until,
        selection,
    )
    .fetch_optional(exec)
    .await
}
