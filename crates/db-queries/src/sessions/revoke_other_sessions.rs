use ids::{SessionId, UserId};
use sqlx::PgExecutor;

/// "Log out everywhere else": revoke all of the user's live sessions except `keep`. Used after a
/// password change (auth.md §3). Returns the number of sessions revoked.
pub async fn revoke_other_sessions<'e, E: PgExecutor<'e>>(
  exec: E,
  user_id: &UserId,
  keep: &SessionId,
) -> sqlx::Result<u64> {
  let result = sqlx::query!(
    r#"UPDATE sessions SET revoked_at = now()
       WHERE user_id = $1 AND id <> $2 AND revoked_at IS NULL"#,
    user_id.as_str(),
    keep.as_str(),
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected())
}
