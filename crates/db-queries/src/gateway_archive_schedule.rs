use ids::{GatewayArchiveId, GatewayExportId};
use sqlx::PgPool;

struct Range {
    account_id: String,
    project_id: String,
    after_seq: i64,
    through_seq: i64,
}

/// Queue the oldest eligible project's next contiguous range. The last verified archive
/// is its checkpoint. A failed or leased archive keeps later ranges behind that checkpoint.
pub async fn queue(pool: &PgPool, age_days: i32, partition_size: i64) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    let range = sqlx::query_as!(
        Range,
        r#"WITH cutoff AS (
            SELECT date_trunc('day', statement_timestamp(), 'UTC')
                   - make_interval(secs => $1::integer::double precision * 86400) AS at
         ), ready AS (
            SELECT h.account_id, h.project_id, coalesce(latest.through_seq, 0) AS after_seq,
                   least(h.watermark, coalesce(latest.through_seq, 0) + $2) AS edge
            FROM gateway_export_heads h
            LEFT JOIN LATERAL (
                SELECT a.through_seq, a.stored_at FROM gateway_archives a
                WHERE a.account_id = h.account_id AND a.project_id = h.project_id AND a.automatic
                ORDER BY a.after_seq DESC LIMIT 1
            ) latest ON true
            JOIN gateway_export_events next_event ON next_event.account_id = h.account_id
                AND next_event.project_id = h.project_id
                AND next_event.published_seq = coalesce(latest.through_seq, 0) + 1
            CROSS JOIN cutoff
            WHERE (latest.through_seq IS NULL OR latest.stored_at IS NOT NULL)
                AND next_event.occurred_at < cutoff.at
            ORDER BY next_event.occurred_at, h.project_id
            LIMIT 1 FOR UPDATE OF h SKIP LOCKED
         )
         SELECT r.account_id, r.project_id, r.after_seq AS "after_seq!",
                coalesce((
                    SELECT min(e.published_seq) - 1 FROM gateway_export_events e
                    WHERE e.account_id = r.account_id AND e.project_id = r.project_id
                      AND e.published_seq > r.after_seq AND e.published_seq <= r.edge
                      AND e.occurred_at >= cutoff.at
                ), r.edge) AS "through_seq!"
         FROM ready r CROSS JOIN cutoff"#,
        age_days,
        partition_size,
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(range) = range else {
        return tx.commit().await;
    };

    let snapshot = GatewayExportId::generate();
    sqlx::query!(
        "INSERT INTO gateway_export_snapshots
            (id, account_id, project_id, requested_by, after_seq, through_seq,
             pending_at_creation, automatic_archive)
         VALUES ($1, $2, $3, 'system', $4, $5, false, true)",
        snapshot.as_str(), range.account_id, range.project_id, range.after_seq, range.through_seq,
    )
    .execute(&mut *tx)
    .await?;
    let archive = GatewayArchiveId::generate();
    let queued = sqlx::query!(
        "INSERT INTO gateway_archives
            (id, account_id, project_id, export_id, requested_by, after_seq, through_seq, automatic)
         VALUES ($1, $2, $3, $4, 'system', $5, $6, true)
         ON CONFLICT (account_id, project_id, after_seq) WHERE automatic DO NOTHING",
        archive.as_str(), range.account_id, range.project_id, snapshot.as_str(),
        range.after_seq, range.through_seq,
    )
    .execute(&mut *tx)
    .await?;
    if queued.rows_affected() == 0 {
        return tx.rollback().await;
    }
    tx.commit().await
}
