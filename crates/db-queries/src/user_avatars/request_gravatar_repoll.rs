use ids::UserId;
use sqlx::PgExecutor;

/// Ask the gravatar-job to fetch this user's gravatar again: reset `gravatar_checked_at` to NULL
/// (the job's work signal). The current mirror keeps displaying until the re-poll lands — the job
/// either replaces it (new mirror) or clears it (gravatar deleted remotely). Returns rows
/// affected.
pub async fn request_gravatar_repoll<'e, E: PgExecutor<'e>>(exec: E, user_id: &UserId) -> sqlx::Result<u64> {
  let result = sqlx::query!(
    r#"UPDATE user_avatars
       SET gravatar_checked_at = NULL, updated_at = now()
       WHERE user_id = $1"#,
    user_id.as_str(),
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected())
}
