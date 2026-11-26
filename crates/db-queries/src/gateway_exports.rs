use chrono::{DateTime, Utc};
use ids::{GatewayExportId, GatewayProjectId};
use serde::{Deserialize, Serialize};
use sqlx::{PgExecutor, PgPool};

use crate::gateway_requests::RequestScope;

pub async fn publish(pool: &PgPool, project: Option<&GatewayProjectId>) -> sqlx::Result<i32> {
    sqlx::query_scalar!(
        r#"SELECT publish_gateway_export_events(1000, $1) AS "count!""#,
        project.map(GatewayProjectId::as_str),
    )
    .fetch_one(pool)
    .await
}

#[derive(Serialize)]
pub struct Snapshot {
    pub id: String,
    pub project_id: String,
    pub client_ref: Option<String>,
    pub after_event_id: Option<String>,
    pub pending_at_creation: bool,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    #[serde(skip)]
    pub after_seq: i64,
    #[serde(skip)]
    pub through_seq: i64,
}

pub async fn create(
    pool: &PgPool,
    scope: RequestScope<'_>,
    id: &GatewayExportId,
    actor_id: &str,
    after: Option<&str>,
) -> sqlx::Result<Option<Snapshot>> {
    sqlx::query_as!(
        Snapshot,
        "WITH cut AS (
           SELECT coalesce((SELECT watermark FROM gateway_export_heads WHERE account_id = $2 AND project_id = $3), 0) AS through_seq,
             CASE WHEN $6::text IS NULL THEN 0 ELSE (
               SELECT published_seq FROM gateway_export_events
               WHERE account_id = $2 AND project_id = $3 AND ($5::text IS NULL OR client_ref = $5) AND id = $6
             ) END AS after_seq,
             EXISTS (SELECT 1 FROM gateway_export_events WHERE account_id = $2 AND project_id = $3
               AND ($5::text IS NULL OR client_ref = $5) AND published_seq IS NULL) AS pending
         )
         INSERT INTO gateway_export_snapshots
           (id, account_id, project_id, requested_by, client_ref, after_event_id, after_seq, through_seq, pending_at_creation)
         SELECT $1, $2, $3, $4, $5, $6, after_seq, through_seq, pending FROM cut WHERE after_seq IS NOT NULL
         RETURNING id, project_id, client_ref, after_event_id, pending_at_creation, created_at, expires_at, after_seq, through_seq",
        id.as_str(), scope.account.as_str(), scope.project.as_str(), actor_id, scope.client_ref, after,
    )
    .fetch_optional(pool)
    .await
}

pub async fn snapshots(
    pool: &PgPool,
    scope: RequestScope<'_>,
    id: Option<&GatewayExportId>,
    before: Option<&GatewayExportId>,
    limit: i64,
) -> sqlx::Result<Vec<Snapshot>> {
    sqlx::query_as!(
        Snapshot,
        "SELECT id, project_id, client_ref, after_event_id, pending_at_creation, created_at, expires_at, after_seq, through_seq
         FROM gateway_export_snapshots
         WHERE account_id = $1 AND project_id = $2 AND ($3::text IS NULL OR client_ref = $3)
           AND ($4::text IS NULL OR id = $4) AND ($5::text IS NULL OR id < $5) AND expires_at > clock_timestamp()
           AND NOT automatic_archive
         ORDER BY id DESC LIMIT $6",
        scope.account.as_str(), scope.project.as_str(), scope.client_ref, id.map(GatewayExportId::as_str),
        before.map(GatewayExportId::as_str), limit,
    )
    .fetch_all(pool)
    .await
}

#[derive(Deserialize, Serialize)]
pub struct ExportEvent {
    pub id: String,
    pub project_id: String,
    pub request_id: String,
    pub client_ref: Option<String>,
    #[serde(rename = "type")]
    pub event_type: String,
    pub occurred_at: DateTime<Utc>,
    pub data: Option<serde_json::Value>,
    #[serde(skip)]
    pub archive_id: Option<String>,
}

pub async fn events(
    pool: &PgPool,
    scope: RequestScope<'_>,
    snapshot: &Snapshot,
    after: Option<&str>,
    limit: i64,
) -> sqlx::Result<Option<Vec<ExportEvent>>> {
    let Some(start) = resolve_cursor(pool, scope, snapshot, after).await? else {
        return Ok(None);
    };
    let scope = RequestScope {
        client_ref: snapshot.client_ref.as_deref(),
        ..scope
    };
    Ok(Some(
        events_between(pool, scope, start, snapshot.through_seq, limit).await?,
    ))
}

pub async fn resolve_cursor(
    pool: &PgPool,
    scope: RequestScope<'_>,
    snapshot: &Snapshot,
    after: Option<&str>,
) -> sqlx::Result<Option<i64>> {
    match after {
        Some(after) => scoped_sequence(
            pool,
            RequestScope {
                client_ref: snapshot.client_ref.as_deref(),
                ..scope
            },
            after,
        )
        .await
        .map(|sequence| {
            sequence.filter(|seq| (snapshot.after_seq..=snapshot.through_seq).contains(seq))
        }),
        None => Ok(Some(snapshot.after_seq)),
    }
}

async fn scoped_sequence<'e, E: PgExecutor<'e>>(
    exec: E,
    scope: RequestScope<'_>,
    after: &str,
) -> sqlx::Result<Option<i64>> {
    sqlx::query_scalar!(
        r#"SELECT published_seq AS "sequence!" FROM gateway_export_events
           WHERE account_id = $1 AND project_id = $2 AND ($3::text IS NULL OR client_ref = $3)
             AND id = $4 AND published_seq IS NOT NULL"#,
        scope.account.as_str(),
        scope.project.as_str(),
        scope.client_ref,
        after,
    )
    .fetch_optional(exec)
    .await
}

pub struct FeedPage {
    pub events: Vec<ExportEvent>,
    pub as_of: DateTime<Utc>,
    pub publication_pending: bool,
}

pub async fn feed_page(
    pool: &PgPool,
    scope: RequestScope<'_>,
    after: Option<&str>,
    fetch_limit: i64,
) -> sqlx::Result<Option<FeedPage>> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await?;
    let start = if let Some(cursor) = after {
        let Some(sequence) = scoped_sequence(&mut *tx, scope, cursor).await? else {
            return Ok(None);
        };
        sequence
    } else {
        0
    };
    let events = events_between(&mut *tx, scope, start, i64::MAX, fetch_limit).await?;
    let status = sqlx::query!(
        r#"SELECT transaction_timestamp() AS "as_of!",
                 EXISTS (SELECT 1 FROM gateway_export_events
                   WHERE account_id = $1 AND project_id = $2 AND published_seq IS NULL
                     AND ($3::text IS NULL OR client_ref = $3)) AS "publication_pending!""#,
        scope.account.as_str(),
        scope.project.as_str(),
        scope.client_ref,
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Some(FeedPage {
        events,
        as_of: status.as_of,
        publication_pending: status.publication_pending,
    }))
}

pub async fn events_between<'e, E: PgExecutor<'e>>(
    exec: E,
    scope: RequestScope<'_>,
    start: i64,
    end: i64,
    limit: i64,
) -> sqlx::Result<Vec<ExportEvent>> {
    sqlx::query_as!(
        ExportEvent,
        "SELECT id, project_id, request_id, client_ref, event_type, occurred_at, data, archive_id FROM gateway_export_events
         WHERE account_id = $1 AND project_id = $2 AND ($3::text IS NULL OR client_ref = $3)
           AND published_seq > $4 AND published_seq <= $5
         ORDER BY published_seq LIMIT $6",
        scope.account.as_str(), scope.project.as_str(), scope.client_ref, start, end, limit,
    )
    .fetch_all(exec)
    .await
}
