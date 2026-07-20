use ids::UserId;
use sqlx::PgExecutor;

/// Set the user's display preference (`custom` / `gravatar` / `letter`). The caller validates
/// the choice makes sense (e.g. `custom` requires an upload to exist); the CHECK constraint is
/// the last line of defense against invalid values. Returns rows affected.
pub async fn set_preferred_source<'e, E: PgExecutor<'e>>(
  exec: E,
  user_id: &UserId,
  preferred_source: &str,
) -> sqlx::Result<u64> {
  let result = sqlx::query!(
    r#"UPDATE user_avatars
       SET preferred_source = $2, updated_at = now()
       WHERE user_id = $1"#,
    user_id.as_str(),
    preferred_source,
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected())
}
