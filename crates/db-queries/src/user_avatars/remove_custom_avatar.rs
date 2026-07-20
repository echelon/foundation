use ids::UserId;
use sqlx::PgExecutor;

/// Remove the custom avatar (the file rows are immutable and kept; only the pointer clears).
/// If it was the preferred display, fall back to `gravatar` when a mirror exists, else the
/// letter monogram. Returns rows affected.
pub async fn remove_custom_avatar<'e, E: PgExecutor<'e>>(exec: E, user_id: &UserId) -> sqlx::Result<u64> {
  let result = sqlx::query!(
    r#"UPDATE user_avatars
       SET custom_file_id = NULL,
           preferred_source = CASE
                                WHEN preferred_source = 'custom' AND gravatar_file_id IS NOT NULL THEN 'gravatar'
                                WHEN preferred_source = 'custom' THEN 'letter'
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
