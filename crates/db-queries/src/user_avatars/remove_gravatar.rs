use ids::UserId;
use sqlx::PgExecutor;

/// Remove the mirrored gravatar (the file row is kept — immutable). `gravatar_checked_at`
/// stays stamped so the job does NOT immediately re-mirror; an explicit refresh request brings
/// it back. If it was the preferred display, fall back to `custom` when an upload exists, else
/// the letter monogram. Returns rows affected.
pub async fn remove_gravatar<'e, E: PgExecutor<'e>>(exec: E, user_id: &UserId) -> sqlx::Result<u64> {
  let result = sqlx::query!(
    r#"UPDATE user_avatars
       SET gravatar_file_id = NULL,
           preferred_source = CASE
                                WHEN preferred_source = 'gravatar' AND custom_file_id IS NOT NULL THEN 'custom'
                                WHEN preferred_source = 'gravatar' THEN 'letter'
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
