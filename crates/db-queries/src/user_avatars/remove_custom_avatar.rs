use ids::UserId;
use sqlx::PgExecutor;

/// Remove the custom avatar (the file rows are immutable and kept; only the pointer clears).
/// If it was the preferred display, the preference resets to `not_yet_set` — the system
/// auto-picks again (gravatar when mirrored, else the monogram). Returns rows affected.
pub async fn remove_custom_avatar<'e, E: PgExecutor<'e>>(exec: E, user_id: &UserId) -> sqlx::Result<u64> {
  let result = sqlx::query!(
    r#"UPDATE user_avatars
       SET custom_file_id = NULL,
           preferred_source = CASE
                                WHEN preferred_source = 'custom' THEN 'not_yet_set'
                                ELSE preferred_source
                              END,
           updated_at = now()
       WHERE user_id = $1"#,
    user_id.as_str(),
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected())
}
