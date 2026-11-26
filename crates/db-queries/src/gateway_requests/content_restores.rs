use chrono::{DateTime, Utc};
use ids::GatewayRequestId;
use sqlx::{PgExecutor, Postgres, Transaction};

use super::RequestScope;

pub const MAX_RESTORE_SUBMISSIONS: i16 = 3;

#[derive(sqlx::FromRow)]
pub struct Progress {
    pub state: String,
    pub ready_objects: i32,
    pub total_objects: i32,
    pub requested_at: DateTime<Utc>,
    pub retry_at: DateTime<Utc>,
}

pub async fn progress(
    conn: impl PgExecutor<'_>,
    request: &GatewayRequestId,
) -> sqlx::Result<Option<Progress>> {
    sqlx::query_as(
        "SELECT CASE WHEN bool_or(state = 'failed') THEN 'failed'
                     WHEN bool_and(state = 'ready') THEN 'ready'
                     WHEN max(requested_at) <= statement_timestamp() - interval '48 hours' THEN 'failed'
                     ELSE 'pending' END AS state,
                (count(*) FILTER (WHERE state = 'ready'))::integer AS ready_objects,
                count(*)::integer AS total_objects, max(requested_at) AS requested_at,
                max(requested_at) + interval '24 hours' AS retry_at
           FROM gateway_content_restores WHERE request_id = $1 HAVING count(*) > 0",
    )
    .bind(request.as_str())
    .fetch_optional(conn)
    .await
}

pub async fn lock_archive(
    tx: &mut Transaction<'_, Postgres>,
    scope: RequestScope<'_>,
    request: &GatewayRequestId,
) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar(
        "SELECT c.target FROM gateway_content_history c
         JOIN gateway_requests r ON r.id = c.request_id
         WHERE r.id = $1 AND r.account_id = $2 AND r.project_id = $3
           AND ($4::text IS NULL OR r.client_ref = $4)
           AND r.content_removed_at IS NULL AND c.digest IS NOT NULL
           AND c.expires_at > clock_timestamp()
         FOR UPDATE OF r",
    )
    .bind(request.as_str())
    .bind(scope.account.as_str())
    .bind(scope.project.as_str())
    .bind(scope.client_ref)
    .fetch_optional(&mut **tx)
    .await
}

pub async fn enqueue(
    tx: &mut Transaction<'_, Postgres>,
    request: &GatewayRequestId,
    manifest_limit: i32,
) -> sqlx::Result<bool> {
    let request = request.as_str();
    // The caller holds the request lock, shared with expiry and explicit removal.
    let blocked: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM gateway_content_restores WHERE request_id = $1
            AND (requested_at > clock_timestamp() - interval '24 hours'
                 OR (state = 'pending' AND requested_at > clock_timestamp() - interval '48 hours'
                     AND NOT EXISTS (SELECT 1 FROM gateway_content_restores
                         WHERE request_id = $1 AND state = 'failed'))))",
    )
    .bind(request)
    .fetch_one(&mut **tx)
    .await?;
    if blocked {
        return Ok(false);
    }
    sqlx::query("DELETE FROM gateway_content_restores WHERE request_id = $1")
        .bind(request)
        .execute(&mut **tx)
        .await?;
    sqlx::query(
        "INSERT INTO gateway_content_restores (request_id, slot, checksum, byte_limit)
         SELECT request_id, -1::smallint, digest, $2 FROM gateway_content_history WHERE request_id = $1
         UNION ALL
         SELECT request_id, slot, archive_sha256, byte_length + 28 FROM gateway_artifacts
         WHERE request_id = $1 AND archive_sha256 IS NOT NULL",
    )
    .bind(request)
    .bind(manifest_limit)
    .execute(&mut **tx)
    .await?;
    Ok(true)
}

#[derive(sqlx::FromRow)]
pub struct Task {
    pub request_id: String,
    pub slot: i16,
    pub requested_at: DateTime<Utc>,
    pub checksum: String,
    pub byte_limit: i32,
    pub attempted: bool,
    pub retrieval_submissions: i16,
    pub retrieval_confirmed: bool,
}

pub async fn claim(tx: &mut Transaction<'_, Postgres>, target: &str) -> sqlx::Result<Option<Task>> {
    sqlx::query_as(
        "SELECT j.request_id, j.slot, j.requested_at, j.checksum, j.byte_limit, j.attempted,
                j.retrieval_submissions, j.retrieval_confirmed
         FROM gateway_content_restores j JOIN gateway_content_history c USING (request_id)
         JOIN gateway_requests r ON r.id = j.request_id
         WHERE c.target = $1 AND c.expires_at > clock_timestamp() AND r.content_removed_at IS NULL
           AND j.state = 'pending' AND j.check_at <= statement_timestamp()
           AND j.requested_at > statement_timestamp() - interval '48 hours'
           AND NOT EXISTS (SELECT 1 FROM gateway_content_restores other
                           WHERE other.request_id = j.request_id AND other.state = 'failed')
         ORDER BY j.check_at, j.request_id, j.slot LIMIT 1 FOR UPDATE OF r, j SKIP LOCKED",
    )
    .bind(target)
    .fetch_optional(&mut **tx)
    .await
}

pub enum Update {
    Wait,
    Confirmed,
    Ready,
    Failed,
}

pub async fn update(tx: &mut Transaction<'_, Postgres>, task: &Task, update: Update) -> sqlx::Result<()> {
    let (state, confirmed) = match update {
        Update::Wait => ("pending", false),
        Update::Confirmed => ("pending", true),
        Update::Ready => ("ready", false),
        Update::Failed => ("failed", false),
    };
    sqlx::query(
        "UPDATE gateway_content_restores SET state = $4, retrieval_confirmed = retrieval_confirmed OR $5,
                check_at = clock_timestamp() + interval '5 minutes'
         WHERE request_id = $1 AND slot = $2 AND requested_at = $3",
    )
    .bind(&task.request_id)
    .bind(task.slot)
    .bind(task.requested_at)
    .bind(state)
    .bind(confirmed)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Reserve one submission under the same request lock used by the restore claim.
pub async fn reserve(tx: &mut Transaction<'_, Postgres>, task: &Task) -> sqlx::Result<Option<i16>> {
    sqlx::query_scalar(
        "UPDATE gateway_content_restores
         SET attempted = true, retrieval_submissions = greatest(retrieval_submissions, attempted::int) + 1,
             check_at = clock_timestamp() + interval '5 minutes'
         WHERE request_id = $1 AND slot = $2 AND requested_at = $3
           AND state = 'pending' AND check_at <= clock_timestamp()
           AND NOT retrieval_confirmed AND retrieval_submissions < $4
         RETURNING retrieval_submissions",
    )
    .bind(&task.request_id)
    .bind(task.slot)
    .bind(task.requested_at)
    .bind(MAX_RESTORE_SUBMISSIONS)
    .fetch_optional(&mut **tx)
    .await
}

pub async fn can_dispatch(
    tx: &mut Transaction<'_, Postgres>,
    target: &str,
    task: &Task,
    submission: i16,
) -> sqlx::Result<bool> {
    // Read object state after taking the request lock, and keep the lock until the POST ends.
    let locked: Option<String> = sqlx::query_scalar(
        "SELECT id FROM gateway_requests WHERE id = $1 AND content_removed_at IS NULL FOR UPDATE",
    )
    .bind(&task.request_id)
    .fetch_optional(&mut **tx)
    .await?;
    if locked.is_none() {
        return Ok(false);
    }
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM gateway_requests r JOIN gateway_content_history c ON c.request_id = r.id
         JOIN gateway_content_restores j ON j.request_id = r.id
         WHERE r.id = $1 AND j.slot = $2 AND j.requested_at = $3 AND c.target = $4
           AND j.state = 'pending' AND j.retrieval_submissions = $5 AND NOT j.retrieval_confirmed
           AND j.check_at > clock_timestamp() AND r.content_removed_at IS NULL
           AND c.expires_at > clock_timestamp()
           AND j.requested_at > clock_timestamp() - interval '48 hours'
           AND NOT EXISTS (SELECT 1 FROM gateway_content_restores other
                           WHERE other.request_id = j.request_id AND other.state = 'failed'))",
    )
    .bind(&task.request_id)
    .bind(task.slot)
    .bind(task.requested_at)
    .bind(target)
    .bind(submission)
    .fetch_one(&mut **tx)
    .await
}
