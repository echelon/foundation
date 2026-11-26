use chrono::{DateTime, Utc};
use ids::GatewayRequestId;
use sqlx::{PgPool, Postgres, Transaction};

use super::{remove_content::remove_locked_content, RequestScope};

pub async fn enroll(
    tx: &mut Transaction<'_, Postgres>,
    id: &GatewayRequestId,
    target: &str,
    input: &[u8],
) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO gateway_content_history (request_id, target, input) VALUES ($1, $2, $3)",
        id.as_str(),
        target,
        input,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[derive(sqlx::FromRow)]
pub struct History {
    pub phase: String,
    pub media: bool,
    pub archive_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub target: String,
    pub digest: Option<String>,
    pub input: Option<Vec<u8>>,
    pub output: Option<Vec<u8>>,
}

pub async fn read(
    pool: &PgPool,
    scope: RequestScope<'_>,
    id: &GatewayRequestId,
    include_content: bool,
) -> sqlx::Result<Option<History>> {
    let row = sqlx::query_as::<_, History>(
        "WITH scoped AS (
            SELECT CASE
                WHEN c.expires_at <= statement_timestamp() THEN 'expired'
                WHEN r.content_removed_at IS NOT NULL THEN 'removed'
                WHEN c.archive_at IS NULL THEN 'processing'
                WHEN c.archive_at > statement_timestamp() THEN 'available'
                WHEN c.digest IS NULL THEN 'archiving'
                ELSE 'archived' END AS phase,
                c.archive_at, c.expires_at, c.target, c.digest, c.input, p.response_payload,
                r.adapter = 'connector_queue' AS media
            FROM gateway_content_history c
            JOIN gateway_requests r ON r.id = c.request_id
            JOIN gateway_provider_runs p ON p.request_id = r.id
            WHERE r.id = $1 AND r.account_id = $2 AND r.project_id = $3
              AND ($4::text IS NULL OR r.client_ref = $4)
         )
         SELECT phase, media, archive_at, expires_at, target, digest,
                CASE WHEN $5::boolean AND phase = 'available' THEN input END AS input,
                CASE WHEN $5::boolean AND phase = 'available' THEN response_payload END AS output
         FROM scoped",
    )
    .bind(id.as_str())
    .bind(scope.account.as_str())
    .bind(scope.project.as_str())
    .bind(scope.client_ref)
    .bind(include_content)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub struct DueArchive {
    pub request_id: String,
    pub media: bool,
    pub input: Vec<u8>,
    pub output: Option<Vec<u8>>,
}

pub async fn claim_archive(
    tx: &mut Transaction<'_, Postgres>,
    target: &str,
) -> sqlx::Result<Option<DueArchive>> {
    sqlx::query_as!(
        DueArchive,
        r#"SELECT c.request_id, c.input AS "input!", p.response_payload AS "output?",
                  (r.adapter = 'connector_queue') AS "media!"
           FROM gateway_content_history c
           JOIN gateway_requests r ON r.id = c.request_id
           JOIN gateway_provider_runs p ON p.request_id = r.id
           WHERE c.target = $1 AND c.input IS NOT NULL AND c.digest IS NULL
             AND r.content_removed_at IS NULL
             AND c.archive_at <= statement_timestamp() AND c.expires_at > statement_timestamp()
             AND (c.retry_at IS NULL OR c.retry_at <= statement_timestamp())
           ORDER BY c.archive_at, c.request_id LIMIT 1 FOR UPDATE OF r SKIP LOCKED"#,
        target,
    )
    .fetch_optional(&mut **tx)
    .await
}

pub async fn archived(
    tx: &mut Transaction<'_, Postgres>,
    id: &str,
    digest: &str,
) -> sqlx::Result<()> {
    sqlx::query!("UPDATE gateway_content_history SET input = NULL, digest = $2, retry_at = NULL WHERE request_id = $1", id, digest)
        .execute(&mut **tx).await?;
    sqlx::query!(
        "UPDATE gateway_provider_runs SET response_payload = NULL WHERE request_id = $1",
        id
    )
    .execute(&mut **tx)
    .await?;
    sqlx::query!("DELETE FROM gateway_chat_deltas WHERE request_id = $1", id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub async fn defer(tx: &mut Transaction<'_, Postgres>, id: &str) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE gateway_content_history SET retry_at = statement_timestamp() + interval '1 minute' WHERE request_id = $1", id,
    ).execute(&mut **tx).await?;
    Ok(())
}

pub async fn expire(pool: &PgPool) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    let ids = sqlx::query_scalar!(
        "SELECT c.request_id FROM gateway_content_history c
         JOIN gateway_requests r ON r.id = c.request_id
         WHERE c.purged_at IS NULL AND c.expires_at <= statement_timestamp() AND r.content_removed_at IS NULL
         ORDER BY c.expires_at, c.request_id LIMIT 64 FOR UPDATE OF r SKIP LOCKED",
    ).fetch_all(&mut *tx).await?;
    if !ids.is_empty() {
        remove_locked_content(&mut tx, &ids).await?;
    }
    tx.commit().await
}

pub struct DuePurge {
    pub request_id: String,
    pub media: bool,
}

pub async fn claim_purge(
    tx: &mut Transaction<'_, Postgres>,
    target: &str,
) -> sqlx::Result<Option<DuePurge>> {
    sqlx::query_as!(
        DuePurge,
        r#"SELECT c.request_id, (r.adapter = 'connector_queue') AS "media!" FROM gateway_content_history c
         JOIN gateway_requests r ON r.id = c.request_id
         WHERE c.target = $1 AND c.purged_at IS NULL AND c.input IS NULL AND r.content_removed_at IS NOT NULL
           AND (c.retry_at IS NULL OR c.retry_at <= statement_timestamp())
         ORDER BY c.request_id LIMIT 1 FOR UPDATE OF r SKIP LOCKED"#,
        target,
    ).fetch_optional(&mut **tx).await
}

pub async fn purged(tx: &mut Transaction<'_, Postgres>, id: &str) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE gateway_content_history SET digest = NULL, purged_at = statement_timestamp(), retry_at = NULL WHERE request_id = $1", id,
    ).execute(&mut **tx).await?;
    Ok(())
}
