use ids::UserId;
use sqlx::PgExecutor;

/// Record the result of a gravatar existence probe (gravatar-job, storage-less fallback path).
/// Always stamps `gravatar_checked_at = now()` so the row leaves the unpolled set.
///
///   - found: promote `source` to `gravatar` unless a custom avatar outranks it.
///   - not found: clear any stale mirror (the user deleted their gravatar remotely) and demote
///     a displaying `gravatar` source back to the monogram.
///
/// Returns rows affected.
pub async fn mark_gravatar_checked<'e, E: PgExecutor<'e>>(
  exec: E,
  user_id: &UserId,
  found: bool,
) -> sqlx::Result<u64> {
  let result = sqlx::query!(
    r#"UPDATE user_avatars
       SET gravatar_checked_at = now(),
           gravatar_file_id = CASE WHEN $2 THEN gravatar_file_id ELSE NULL END,
           source = CASE
                      WHEN $2 AND source <> 'custom' AND NOT is_gravatar_dismissed THEN 'gravatar'
                      WHEN NOT $2 AND source = 'gravatar' THEN 'letter'
                      ELSE source
                    END,
           updated_at = now()
       WHERE user_id = $1"#,
    user_id.as_str(),
    found,
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected())
}
