use chrono::{DateTime, Utc};
use ids::GatewayRequestId;
use sqlx::PgPool;

use super::RequestScope;

pub struct Delta {
    pub ordinal: i16,
    pub payload: Vec<u8>,
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
) -> sqlx::Result<Vec<Delta>> {
    sqlx::query_as!(
        Delta,
        "SELECT d.ordinal, d.payload
         FROM gateway_chat_deltas d JOIN gateway_requests r ON r.id = d.request_id
         JOIN gateway_provider_runs p ON p.request_id = r.id
         WHERE r.id = $1 AND r.account_id = $2 AND r.project_id = $3
           AND ($4::text IS NULL OR r.client_ref = $4) AND d.ordinal > $5
           AND r.content_removed_at IS NULL
           AND coalesce(p.result_expires_at,
               coalesce(r.succeeded_at, r.failed_at, r.uncertain_at, r.canceled_at, statement_timestamp())
                   + interval '1 hour') > statement_timestamp()
         ORDER BY d.ordinal LIMIT 32",
        request.as_str(), scope.account.as_str(), scope.project.as_str(), scope.client_ref, after,
    ).fetch_all(pool).await
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
