use ids::GatewayRequestId;
use sqlx::{PgConnection, PgPool};

pub async fn reserve(
    pool: &PgPool,
    request: &GatewayRequestId,
    key: &str,
    target: &str,
) -> sqlx::Result<()> {
    sqlx::query("INSERT INTO gateway_artifact_objects (object_key, request_id, target) VALUES ($1, $2, $3)")
        .bind(key)
        .bind(request.as_str())
        .bind(target)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn publish(conn: &mut PgConnection, request: &str, key: &str) -> sqlx::Result<()> {
    // Publish the reference and terminal request in the same transaction. Once a
    // cleaner has claimed a reservation, an overlapping writer must retry.
    let result = sqlx::query(
        "UPDATE gateway_artifact_objects SET cleanup_at = NULL
         WHERE object_key = $1 AND request_id = $2 AND NOT deleting AND cleanup_at IS NOT NULL",
    )
    .bind(key)
    .bind(request)
    .execute(conn)
    .await?;
    if result.rows_affected() != 1 {
        return Err(sqlx::Error::RowNotFound);
    }
    Ok(())
}

#[derive(sqlx::FromRow)]
pub struct DeleteTask {
    pub request_id: String,
    pub object_key: String,
}

pub async fn claim_deletions(pool: &PgPool, target: &str) -> sqlx::Result<Vec<DeleteTask>> {
    sqlx::query_as(
        "WITH due AS (
             SELECT object_key FROM gateway_artifact_objects
             WHERE target = $1 AND cleanup_at <= statement_timestamp()
             ORDER BY cleanup_at, object_key LIMIT 8 FOR UPDATE SKIP LOCKED
         )
         UPDATE gateway_artifact_objects AS objects
         SET deleting = true, cleanup_at = statement_timestamp() + interval '1 minute'
         FROM due WHERE objects.object_key = due.object_key
         RETURNING objects.request_id, objects.object_key",
    )
    .bind(target)
    .fetch_all(pool)
    .await
}

pub async fn deleted(pool: &PgPool, task: &DeleteTask) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM gateway_artifact_objects WHERE object_key = $1 AND deleting")
        .bind(&task.object_key)
        .execute(pool)
        .await?;
    Ok(())
}
