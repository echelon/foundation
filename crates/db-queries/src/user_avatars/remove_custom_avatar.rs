use ids::UserId;
use sqlx::PgExecutor;

/// Remove the custom avatar. Display falls back down the precedence: mirrored, undismissed
/// gravatar if there is one, else the letter monogram (accounts.md §2). The file rows are kept
/// (immutable); only the pointer clears. Returns rows affected.
pub async fn remove_custom_avatar<'e, E: PgExecutor<'e>>(exec: E, user_id: &UserId) -> sqlx::Result<u64> {
  let result = sqlx::query!(
    r#"UPDATE user_avatars
       SET custom_file_id = NULL,
           source = CASE
                      WHEN gravatar_file_id IS NOT NULL AND NOT is_gravatar_dismissed THEN 'gravatar'
                      ELSE 'letter'
                    END,
           updated_at = now()
       WHERE user_id = $1"#,
    user_id.as_str(),
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected())
}
