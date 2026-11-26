use chrono::{DateTime, Utc};
use ids::GatewayRequestId;
use sqlx::{PgPool, Postgres, Transaction};

use super::RequestScope;

pub enum ContentRemoval {
    Missing,
    Ineligible,
    Removed { at: DateTime<Utc>, first: bool },
}

/// Lock the request before touching content, as provider completion and stream writes do,
/// so a concurrent worker cannot restore removed bytes.
pub async fn remove_content(
    tx: &mut Transaction<'_, Postgres>,
    scope: RequestScope<'_>,
    request: &GatewayRequestId,
) -> sqlx::Result<ContentRemoval> {
    let id = request.as_str();
    let target = sqlx::query!(
        r#"SELECT content_removed_at, (adapter <> 'mock' AND
                    num_nonnulls(succeeded_at, failed_at, canceled_at) = 1) AS "eligible!"
           FROM gateway_requests
           WHERE id = $1 AND account_id = $2 AND project_id = $3
             AND ($4::text IS NULL OR client_ref = $4)
           FOR UPDATE"#,
        id,
        scope.account.as_str(),
        scope.project.as_str(),
        scope.client_ref,
    )
    .fetch_optional(&mut **tx)
    .await?;
    let Some(target) = target else {
        return Ok(ContentRemoval::Missing);
    };
    if let Some(at) = target.content_removed_at {
        return Ok(ContentRemoval::Removed { at, first: false });
    }
    if !target.eligible {
        return Ok(ContentRemoval::Ineligible);
    }

    let at = remove_locked_content(tx, &[id.to_owned()]).await?;
    Ok(ContentRemoval::Removed { at, first: true })
}

/// Use the persisted result deadline, including older runs whose bytes were
/// already cleared by the earlier worker. Lock requests in the same order as
/// manual removal and provider completion.
pub async fn expire_results(pool: &PgPool) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    let ids = sqlx::query_scalar!(
        "SELECT p.request_id FROM gateway_provider_runs p
         JOIN gateway_requests r ON r.id = p.request_id
         WHERE p.result_expires_at <= statement_timestamp()
           AND r.content_removed_at IS NULL AND r.succeeded_at IS NOT NULL
           AND NOT EXISTS (SELECT 1 FROM gateway_content_history c WHERE c.request_id = r.id)
         ORDER BY p.result_expires_at, p.request_id LIMIT 64
         FOR UPDATE OF r SKIP LOCKED",
    )
    .fetch_all(&mut *tx)
    .await?;
    if !ids.is_empty() {
        remove_locked_content(&mut tx, &ids).await?;
    }
    tx.commit().await
}

pub(super) async fn remove_locked_content(
    tx: &mut Transaction<'_, Postgres>,
    ids: &[String],
) -> sqlx::Result<DateTime<Utc>> {
    sqlx::query!(
        "UPDATE gateway_provider_runs
         SET request_payload = NULL, response_payload = NULL, result_expires_at = NULL
         WHERE request_id = ANY($1)",
        ids,
    )
    .execute(&mut **tx)
    .await?;
    sqlx::query!("DELETE FROM gateway_chat_deltas WHERE request_id = ANY($1)", ids)
        .execute(&mut **tx)
        .await?;
    sqlx::query!("DELETE FROM gateway_artifacts WHERE request_id = ANY($1)", ids)
        .execute(&mut **tx)
        .await?;
    sqlx::query_scalar!(
        r#"WITH removed AS (
               UPDATE gateway_requests SET content_removed_at = statement_timestamp()
               WHERE id = ANY($1) RETURNING content_removed_at
           )
           SELECT min(content_removed_at) AS "removed_at!" FROM removed"#,
        ids,
    )
    .fetch_one(&mut **tx)
    .await
}
