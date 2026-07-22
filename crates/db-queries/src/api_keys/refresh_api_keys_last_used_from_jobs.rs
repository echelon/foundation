use sqlx::PgExecutor;

/// Out-of-band `last_used_at` refresh (the background job's task): derive each key's true
/// most-recent use from the jobs it created in the last day and advance the stamp where
/// it lags. Keeps the hot auth path read-only without letting `last_used_at` drift more
/// than one sweep behind reality. Returns how many keys were advanced.
pub async fn refresh_api_keys_last_used_from_jobs<'e, E: PgExecutor<'e>>(exec: E) -> sqlx::Result<u64> {
  let result = sqlx::query!(
    r#"UPDATE api_keys k SET last_used_at = j.latest
       FROM (SELECT api_key_id, max(created_at) AS latest
             FROM jobs
             WHERE api_key_id IS NOT NULL AND created_at > now() - interval '1 day'
             GROUP BY api_key_id) j
       WHERE k.id = j.api_key_id AND (k.last_used_at IS NULL OR k.last_used_at < j.latest)"#,
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected())
}
