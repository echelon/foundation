use chrono::{DateTime, Utc};
use ids::GatewayRequestId;
use sqlx::{PgConnection, PgPool};

use super::{artifact_objects, RequestScope};

pub struct NewArtifact {
    pub slot: i16,
    pub content_type: &'static str,
    pub byte_length: i32,
    pub sha256: String,
    pub data: ArtifactData,
}

pub enum ArtifactData {
    Inline(Vec<u8>),
    File(String),
    Bucket(String),
}

pub struct Artifact {
    pub content_type: String,
    pub byte_length: i32,
    pub sha256: String,
    pub payload: Option<Vec<u8>>,
    pub object_key: Option<String>,
    pub bucket_key: Option<String>,
    pub bucket_target: Option<String>,
}

pub(super) async fn insert(
    conn: &mut PgConnection,
    request: &str,
    expires_at: DateTime<Utc>,
    artifacts: &[NewArtifact],
) -> sqlx::Result<()> {
    for artifact in artifacts {
        let (payload, object_key, bucket_key) = match &artifact.data {
            ArtifactData::Inline(bytes) => (Some(bytes.as_slice()), None, None),
            ArtifactData::File(key) => (None, Some(key.as_str()), None),
            ArtifactData::Bucket(key) => {
                artifact_objects::publish(&mut *conn, request, key).await?;
                (None, None, Some(key.as_str()))
            }
        };
        sqlx::query!(
            "INSERT INTO gateway_artifacts
                (request_id, slot, content_type, byte_length, sha256, payload, object_key, bucket_key, expires_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            request,
            artifact.slot,
            artifact.content_type,
            artifact.byte_length,
            artifact.sha256,
            payload,
            object_key,
            bucket_key,
            expires_at,
        )
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

pub async fn read(
    pool: &PgPool,
    scope: RequestScope<'_>,
    request: &GatewayRequestId,
    slot: i16,
    include_payload: bool,
) -> sqlx::Result<Option<Artifact>> {
    sqlx::query_as!(
        Artifact,
        r#"SELECT a.content_type, a.byte_length, a.sha256,
                CASE WHEN $6 THEN a.payload END AS "payload?", a.object_key, a.bucket_key,
                objects.target AS "bucket_target?"
         FROM gateway_artifacts a JOIN gateway_requests r ON r.id = a.request_id
         JOIN gateway_provider_runs p ON p.request_id = r.id
         LEFT JOIN gateway_artifact_objects objects ON objects.object_key = a.bucket_key
         WHERE r.account_id = $1 AND r.project_id = $2 AND ($3::text IS NULL OR r.client_ref = $3)
           AND r.id = $4 AND a.slot = $5 AND r.succeeded_at IS NOT NULL
           AND r.content_removed_at IS NULL AND p.result_expires_at > statement_timestamp()
           AND a.archive_sha256 IS NULL AND a.expires_at > statement_timestamp()"#,
        scope.account.as_str(),
        scope.project.as_str(),
        scope.client_ref,
        request.as_str(),
        slot,
        include_payload,
    )
    .fetch_optional(pool)
    .await
}

pub async fn archive_slots(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    request: &str,
) -> sqlx::Result<Vec<i16>> {
    sqlx::query_scalar!(
        "SELECT slot FROM gateway_artifacts WHERE request_id = $1 ORDER BY slot",
        request,
    )
    .fetch_all(&mut **tx)
    .await
}

pub async fn archive_source(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    request: &str,
    slot: i16,
) -> sqlx::Result<Artifact> {
    sqlx::query_as!(
        Artifact,
        r#"SELECT a.content_type, a.byte_length, a.sha256, a.payload, a.object_key,
                a.bucket_key, objects.target AS "bucket_target?"
         FROM gateway_artifacts a
         LEFT JOIN gateway_artifact_objects objects ON objects.object_key = a.bucket_key
         WHERE a.request_id = $1 AND a.slot = $2 AND a.archive_sha256 IS NULL"#,
        request,
        slot,
    )
    .fetch_one(&mut **tx)
    .await
}

pub async fn mark_archived(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    request: &str,
    slot: i16,
    checksum: &str,
) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE gateway_artifacts SET payload = NULL, object_key = NULL, bucket_key = NULL, archive_sha256 = $3
         WHERE request_id = $1 AND slot = $2",
        request,
        slot,
        checksum,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub struct ArchivedArtifact {
    pub content_type: String,
    pub byte_length: i32,
    pub sha256: String,
    pub archive_sha256: String,
    pub target: String,
}

pub async fn read_archived(
    pool: &PgPool,
    scope: RequestScope<'_>,
    request: &GatewayRequestId,
    slot: i16,
) -> sqlx::Result<Option<ArchivedArtifact>> {
    sqlx::query_as!(
        ArchivedArtifact,
        r#"SELECT a.content_type, a.byte_length, a.sha256, a.archive_sha256 AS "archive_sha256!", c.target
           FROM gateway_artifacts a JOIN gateway_requests r ON r.id = a.request_id
           JOIN gateway_content_history c ON c.request_id = r.id
           WHERE r.account_id = $1 AND r.project_id = $2 AND ($3::text IS NULL OR r.client_ref = $3)
             AND r.id = $4 AND a.slot = $5 AND r.succeeded_at IS NOT NULL AND r.content_removed_at IS NULL
             AND a.archive_sha256 IS NOT NULL AND c.digest IS NOT NULL
             AND c.archive_at <= statement_timestamp() AND c.expires_at > statement_timestamp()
             AND a.expires_at > statement_timestamp()"#,
        scope.account.as_str(), scope.project.as_str(), scope.client_ref, request.as_str(), slot,
    ).fetch_optional(pool).await
}

pub async fn namespace(pool: &PgPool) -> sqlx::Result<String> {
    sqlx::query_scalar!(
        r#"SELECT id::text AS "id!" FROM gateway_artifact_namespace WHERE singleton"#,
    )
    .fetch_one(pool)
    .await
}

pub async fn disposable_files(
    pool: &PgPool,
    requests: &[String],
    keys: &[String],
) -> sqlx::Result<Vec<i64>> {
    sqlx::query_scalar!(
        r#"SELECT candidate.position AS "position!"
           FROM unnest($1::text[], $2::text[]) WITH ORDINALITY
             AS candidate(request_id, object_key, position)
           WHERE NOT EXISTS (
             SELECT 1 FROM gateway_requests r WHERE r.id = candidate.request_id
               AND num_nonnulls(r.succeeded_at, r.failed_at, r.uncertain_at, r.canceled_at) = 0
           ) AND NOT EXISTS (
             SELECT 1 FROM gateway_artifacts a WHERE a.request_id = candidate.request_id
               AND a.object_key = candidate.object_key AND a.expires_at > clock_timestamp()
           )"#,
        requests,
        keys,
    )
    .fetch_all(pool)
    .await
}

pub async fn purge_expired(pool: &PgPool) -> sqlx::Result<()> {
    sqlx::query!(
        "DELETE FROM gateway_artifacts WHERE (request_id, slot) IN (
           SELECT request_id, slot FROM gateway_artifacts
           WHERE expires_at <= clock_timestamp() ORDER BY expires_at LIMIT 64
           FOR UPDATE SKIP LOCKED
         )",
    )
    .execute(pool)
    .await?;
    Ok(())
}
