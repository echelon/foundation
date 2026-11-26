use sqlx::PgExecutor;

pub async fn complete_mock_attempts<'e, E: PgExecutor<'e>>(exec: E) -> sqlx::Result<u64> {
    let result = sqlx::query!(
        "WITH ready AS (
           SELECT a.account_id, a.request_id, a.attempt_number
           FROM gateway_provider_attempts a JOIN gateway_requests r ON r.account_id = a.account_id AND r.id = a.request_id
           WHERE r.adapter = 'mock' AND r.canceled_at IS NULL AND r.succeeded_at IS NULL
             AND a.completed_at IS NULL AND a.canceled_at IS NULL AND a.complete_after <= now()
           ORDER BY a.complete_after, a.request_id LIMIT 16 FOR UPDATE OF r SKIP LOCKED
         ), finished AS (
           UPDATE gateway_requests r SET succeeded_at = now() FROM ready
           WHERE r.account_id = ready.account_id AND r.id = ready.request_id
           RETURNING r.account_id, r.id, r.succeeded_at, r.estimated_usd_nanos, ready.attempt_number,
             coalesce((r.quote #>> '{input,workload,count}')::smallint, 1) AS output_count
         ), reported AS (
           UPDATE gateway_provider_attempts a
           SET completed_at = f.succeeded_at, reported_usd_nanos = f.estimated_usd_nanos
           FROM finished f WHERE a.account_id = f.account_id AND a.request_id = f.id
             AND a.attempt_number = f.attempt_number
           RETURNING a.account_id, a.request_id, a.attempt_number, a.reported_usd_nanos, f.output_count, a.completed_at
         )
         INSERT INTO gateway_usage_observations
           (account_id, request_id, attempt_number, source, amount_usd_nanos, output_count, reported_at)
         SELECT account_id, request_id, attempt_number, 'mock_provider', reported_usd_nanos, output_count, completed_at
         FROM reported",
    ).execute(exec).await?;
    Ok(result.rows_affected())
}
