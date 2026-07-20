use ids::UserId;
use sqlx::PgExecutor;

/// Record the result of a gravatar existence probe (gravatar-job). Always stamps
/// `gravatar_checked_at = now()` so the row leaves the unpolled set. When a gravatar exists and the
/// user hasn't set a custom avatar, promote `source` to `gravatar` so consoles display it.
///
/// NOTE: byte-mirroring into `user_avatar_files` (accounts.md §2) needs object storage that isn't
/// wired yet; until then this records existence only. Returns rows affected.
pub async fn mark_gravatar_checked<'e, E: PgExecutor<'e>>(
  exec: E,
  user_id: &UserId,
  found: bool,
) -> sqlx::Result<u64> {
  let result = sqlx::query!(
    r#"UPDATE user_avatars
       SET gravatar_checked_at = now(),
           source = CASE
                      WHEN $2 AND source <> 'custom' THEN 'gravatar'
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
