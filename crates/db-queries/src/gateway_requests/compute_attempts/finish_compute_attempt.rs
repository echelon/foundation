use sqlx::PgExecutor;

use super::ComputeAttemptLease;

pub struct ComputeAttemptFinish<'a> {
    pub lease: ComputeAttemptLease<'a>,
    pub slot: i32,
    pub result: &'a serde_json::Value,
}

pub async fn finish_compute_attempt<'e, E: PgExecutor<'e>>(
    exec: E,
    finish: ComputeAttemptFinish<'_>,
) -> sqlx::Result<bool> {
    let changed = sqlx::query!(
        r#"UPDATE gateway_provider_runs p
           SET compute_attempts = jsonb_set(
               p.compute_attempts, ARRAY[($3::int - 1)::text, 'result'],
               $4::jsonb || jsonb_build_object('observed_at', clock_timestamp())
           )
           WHERE p.request_id = $1 AND p.lease_until = $2 AND p.lease_until > clock_timestamp()
             AND $3 BETWEEN 1 AND jsonb_array_length(p.compute_attempts)
             AND p.compute_attempts -> ($3 - 1) -> 'result' IS NULL"#,
        finish.lease.request.as_str(),
        finish.lease.until,
        finish.slot,
        finish.result,
    )
    .execute(exec)
    .await?;
    Ok(changed.rows_affected() == 1)
}
