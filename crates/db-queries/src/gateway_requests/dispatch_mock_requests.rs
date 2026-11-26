use sqlx::PgPool;

use super::queue_turns::QueueLane;

/// Schedule a deterministic fixture and record its handle in one statement. There is no external IO.
pub async fn dispatch_mock_requests(pool: &PgPool) -> sqlx::Result<u64> {
    let lane = QueueLane::MockSubmit;
    let Some(mut tx) = lane.begin(pool).await? else {
        return Ok(0);
    };
    let result = sqlx::query!(
        "WITH ranked AS (
           SELECT id, row_number() OVER (PARTITION BY project_id ORDER BY dispatch_after, sid) AS project_position
           FROM gateway_requests r
           WHERE adapter = 'mock' AND dispatched_at IS NULL AND canceled_at IS NULL AND dispatch_after <= now()
             AND NOT EXISTS (
                 SELECT 1 FROM gateway_dispatch_controls control WHERE control.account_id = r.account_id
                   AND (control.pause_all OR r.provider = ANY(control.paused_providers))
             )
         ), ready AS (
           SELECT r.id FROM ranked
           JOIN gateway_requests r ON r.id = ranked.id
           LEFT JOIN gateway_queue_turns q ON q.project_id = r.project_id AND q.lane = $1
           WHERE r.dispatched_at IS NULL AND r.canceled_at IS NULL
           ORDER BY ranked.project_position, coalesce(q.turn, 0), r.dispatch_after, r.sid
           LIMIT 16 FOR UPDATE OF r SKIP LOCKED
         ), dispatched AS (
           UPDATE gateway_requests r SET dispatched_at = now() FROM ready WHERE r.id = ready.id
           RETURNING r.account_id, r.project_id, r.id, r.provider, r.mock_latency_ms, r.dispatched_at
         ), served AS (
           INSERT INTO gateway_queue_turns (project_id, lane)
           SELECT DISTINCT project_id, $1 FROM dispatched
           ON CONFLICT (project_id, lane) DO UPDATE SET turn = excluded.turn
         )
         INSERT INTO gateway_provider_attempts
           (account_id, request_id, attempt_number, provider_handle, submitted_at, complete_after)
         SELECT account_id, id, 1, 'mock_' || provider || '_' || id, dispatched_at,
                dispatched_at + mock_latency_ms * interval '1 millisecond'
         FROM dispatched",
        lane.key(),
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(result.rows_affected())
}
