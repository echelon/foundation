use ids::UserId;
use sqlx::PgExecutor;

/// Record the result of a gravatar probe (gravatar-job). Always stamps `gravatar_checked_at`
/// so the row leaves the unpolled set. When the address has no gravatar (`found = false`), any
/// stale mirror is cleared — the user deleted their gravatar remotely — and a `gravatar`
/// preference simply renders the monogram until one exists again. The preference itself is
/// never touched here: display is derived, the choice is the user's. Returns rows affected.
pub async fn mark_gravatar_checked<'e, E: PgExecutor<'e>>(
  exec: E,
  user_id: &UserId,
  found: bool,
) -> sqlx::Result<u64> {
  let result = sqlx::query!(
    r#"UPDATE user_avatars
       SET gravatar_checked_at = now(),
           gravatar_file_id = CASE WHEN $2 THEN gravatar_file_id ELSE NULL END,
           updated_at = now()
       WHERE user_id = $1"#,
    user_id.as_str(),
    found,
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected())
}
