use chrono::{DateTime, Utc};
use ids::GatewayRequestId;
use sqlx::PgPool;

use super::{RequestScope, RequestStatus};

pub const PAGE_SIZE: usize = 32;

pub struct Delta {
    pub ordinal: i16,
    pub payload: Vec<u8>,
}

pub struct StreamPage {
    pub status: RequestStatus,
    pub created_at: DateTime<Utc>,
    pub succeeded_at: Option<DateTime<Utc>>,
    pub content_removed_at: Option<DateTime<Utc>>,
    pub token_counts: Option<(i32, i32)>,
    pub chunks: Vec<Delta>,
}

struct StreamRow {
    status: RequestStatus,
    created_at: DateTime<Utc>,
    succeeded_at: Option<DateTime<Utc>>,
    content_removed_at: Option<DateTime<Utc>>,
    input_tokens: Option<i32>,
    output_tokens: Option<i32>,
    ordinal: Option<i16>,
    payload: Option<Vec<u8>>,
}

pub async fn append(
    pool: &PgPool,
    request: &str,
    lease: DateTime<Utc>,
    ordinal: i16,
    payload: &[u8],
    upstream_handle: &str,
) -> sqlx::Result<bool> {
    let changed = sqlx::query!(
        "WITH live AS (
           SELECT r.id FROM gateway_requests r JOIN gateway_provider_runs p ON p.request_id = r.id
           WHERE r.id = $1 AND r.adapter IN ('openrouter_chat', 'openai_compatible_chat') AND p.lease_until = $2
             AND p.lease_until > clock_timestamp()
             AND num_nonnulls(r.succeeded_at, r.failed_at, r.uncertain_at, r.canceled_at) = 0
           FOR UPDATE OF r
         ), saved AS (
           INSERT INTO gateway_chat_deltas (request_id, ordinal, payload)
           SELECT id, $3, $4 FROM live RETURNING request_id, stored_at
         )
         UPDATE gateway_provider_runs p SET stream_started_at = coalesce(p.stream_started_at, saved.stored_at),
             upstream_handle = coalesce(p.upstream_handle, $5)
         FROM saved WHERE p.request_id = saved.request_id",
        request, lease, ordinal, payload, upstream_handle,
    ).execute(pool).await?;
    Ok(changed.rows_affected() == 1)
}

pub async fn read(
    pool: &PgPool,
    scope: RequestScope<'_>,
    request: &GatewayRequestId,
    after: i16,
) -> sqlx::Result<Option<StreamPage>> {
    // Read lifecycle and deltas from one scoped statement snapshot. A poll does
    // not need the stored quote, provider lineage, or settlement history.
    let rows = sqlx::query_as!(
        StreamRow,
        r#"SELECT CASE
                    WHEN r.uncertain_at IS NOT NULL THEN 'awaiting_reconciliation'
                    WHEN r.failed_at IS NOT NULL THEN 'failed'
                    WHEN r.canceled_at IS NOT NULL THEN 'canceled'
                    WHEN r.succeeded_at IS NOT NULL THEN 'succeeded'
                    WHEN r.dispatched_at IS NOT NULL THEN 'running'
                    ELSE 'queued'
                  END AS "status!: RequestStatus",
                  r.created_at, r.succeeded_at, r.content_removed_at,
                  usage.input_tokens AS "input_tokens?", usage.output_tokens AS "output_tokens?",
                  d.ordinal AS "ordinal?", d.payload AS "payload?"
         FROM gateway_requests r
         LEFT JOIN gateway_provider_runs p ON p.request_id = r.id
         LEFT JOIN LATERAL (
           SELECT input_tokens, output_tokens FROM gateway_usage_observations
           WHERE r.succeeded_at IS NOT NULL AND account_id = r.account_id
             AND request_id = r.id AND attempt_number = 1
             AND source IN ('openrouter_report', 'openrouter_stream_partial',
                            'local_chat_report', 'local_chat_stream_partial')
           ORDER BY source LIMIT 1
         ) usage ON true
         LEFT JOIN LATERAL (
           SELECT ordinal, payload FROM gateway_chat_deltas
           WHERE request_id = p.request_id AND ordinal > $5
             AND r.content_removed_at IS NULL
             AND coalesce(p.result_expires_at,
                 coalesce(r.succeeded_at, r.failed_at, r.uncertain_at, r.canceled_at, statement_timestamp())
                     + interval '1 hour') > statement_timestamp()
           ORDER BY ordinal LIMIT $6
         ) d ON true
         WHERE r.id = $1 AND r.account_id = $2 AND r.project_id = $3
           AND ($4::text IS NULL OR r.client_ref = $4)
           AND r.adapter IN ('openrouter_chat', 'openai_compatible_chat')
         ORDER BY d.ordinal"#,
        request.as_str(), scope.account.as_str(), scope.project.as_str(), scope.client_ref, after,
        PAGE_SIZE as i64,
    ).fetch_all(pool).await?;
    let Some(first) = rows.first() else {
        return Ok(None);
    };
    Ok(Some(StreamPage {
        status: first.status,
        created_at: first.created_at,
        succeeded_at: first.succeeded_at,
        content_removed_at: first.content_removed_at,
        token_counts: first.input_tokens.zip(first.output_tokens),
        chunks: rows.into_iter().filter_map(|row| {
            row.ordinal.zip(row.payload).map(|(ordinal, payload)| Delta { ordinal, payload })
        }).collect(),
    }))
}

pub async fn record_detach(pool: &PgPool, request: &GatewayRequestId) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE gateway_provider_runs SET stream_detached_at = clock_timestamp()
         WHERE request_id = $1 AND stream_detached_at IS NULL",
        request.as_str(),
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn purge_expired(pool: &PgPool) -> sqlx::Result<()> {
    sqlx::query!(
        "DELETE FROM gateway_chat_deltas WHERE (request_id, ordinal) IN (
           SELECT d.request_id, d.ordinal FROM gateway_chat_deltas d
           JOIN gateway_requests r ON r.id = d.request_id
           WHERE r.succeeded_at IS NULL AND d.stored_at <= clock_timestamp() - interval '1 hour'
             AND coalesce(r.succeeded_at, r.failed_at, r.uncertain_at, r.canceled_at)
                 <= clock_timestamp() - interval '1 hour'
           ORDER BY d.stored_at LIMIT 64 FOR UPDATE OF d SKIP LOCKED
         )",
    )
    .execute(pool)
    .await?;
    Ok(())
}
