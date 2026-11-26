use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};

pub struct Retirement {
    pub id: String,
    pub account_id: String,
    pub project_id: String,
    pub after_seq: i64,
    pub through_seq: i64,
    pub generation: i32,
    pub check_at: DateTime<Utc>,
}

pub async fn claim(pool: &PgPool, age_days: i32) -> sqlx::Result<Option<Retirement>> {
    sqlx::query_as!(
        Retirement,
        r#"WITH ready AS (
            SELECT id FROM gateway_archives
            WHERE automatic AND retired_at IS NULL AND stored_at IS NOT NULL
              AND stored_at <= statement_timestamp() - make_interval(secs => $1::integer::double precision * 86400)
              AND (retirement_check_at IS NULL OR retirement_check_at <= statement_timestamp())
            ORDER BY stored_at, id LIMIT 1 FOR UPDATE SKIP LOCKED
         )
         UPDATE gateway_archives a SET retirement_check_at = statement_timestamp() + interval '5 minutes'
         FROM ready WHERE a.id = ready.id
         RETURNING a.id, a.account_id, a.project_id, a.after_seq, a.through_seq,
                   a.storage_generation AS "generation!", a.retirement_check_at AS "check_at!""#,
        age_days,
    )
    .fetch_optional(pool)
    .await
}

pub async fn lock(tx: &mut Transaction<'_, Postgres>, task: &Retirement) -> sqlx::Result<bool> {
    let found = sqlx::query_scalar!(
        "SELECT id FROM gateway_archives
         WHERE id = $1 AND retired_at IS NULL AND storage_generation = $2 AND retirement_check_at = $3
         FOR UPDATE",
        task.id, task.generation, task.check_at,
    )
    .fetch_optional(&mut **tx)
    .await?;
    Ok(found.is_some())
}

/// Called inside the same repeatable-read transaction that verified the source payloads.
pub async fn retire(tx: &mut Transaction<'_, Postgres>, task: &Retirement) -> sqlx::Result<bool> {
    let changed = sqlx::query!(
        "UPDATE gateway_export_events SET data = NULL, archive_id = $1
         WHERE account_id = $2 AND project_id = $3 AND published_seq > $4 AND published_seq <= $5
           AND data IS NOT NULL",
        task.id,
        task.account_id,
        task.project_id,
        task.after_seq,
        task.through_seq,
    )
    .execute(&mut **tx)
    .await?;
    if changed.rows_affected() != (task.through_seq - task.after_seq) as u64 {
        return Ok(false);
    }
    sqlx::query!(
        "UPDATE gateway_archives SET retired_at = clock_timestamp(),
            retirement_check_at = NULL, retirement_error_code = NULL WHERE id = $1",
        task.id,
    )
    .execute(&mut **tx)
    .await?;
    Ok(true)
}

pub async fn defer(pool: &PgPool, task: &Retirement, reason: &str) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE gateway_archives SET retirement_error_code = $4
         WHERE id = $1 AND retired_at IS NULL AND storage_generation = $2 AND retirement_check_at = $3",
        task.id, task.generation, task.check_at, reason,
    )
    .execute(pool)
    .await?;
    Ok(())
}
