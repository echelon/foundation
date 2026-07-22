use ids::ApiKeyId;
use sqlx::PgExecutor;

/// Coarse `last_used_at` stamp: writes ONLY when the current value is missing or more
/// than 10 minutes old, so a busy key costs at most one row write per 10 minutes. Finer
/// freshness comes out-of-band ([`refresh_api_keys_last_used_from_jobs`]).
pub async fn touch_api_key_last_used<'e, E: PgExecutor<'e>>(exec: E, id: &ApiKeyId) -> sqlx::Result<bool> {
  let result = sqlx::query!(
    r#"UPDATE api_keys SET last_used_at = now()
       WHERE id = $1 AND (last_used_at IS NULL OR last_used_at < now() - interval '10 minutes')"#,
    id.as_str(),
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected() > 0)
}
