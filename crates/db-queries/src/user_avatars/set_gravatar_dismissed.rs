use ids::UserId;
use sqlx::PgExecutor;

/// Dismiss (or restore) the gravatar — reversible; the mirrored file is kept (accounts.md §2).
/// Display `source` recomputes: dismissing a shown gravatar falls back to the letter monogram;
/// restoring promotes back to `gravatar` when one is mirrored and nothing outranks it.
/// Returns rows affected.
pub async fn set_gravatar_dismissed<'e, E: PgExecutor<'e>>(
  exec: E,
  user_id: &UserId,
  dismissed: bool,
) -> sqlx::Result<u64> {
  let result = sqlx::query!(
    r#"UPDATE user_avatars
       SET is_gravatar_dismissed = $2,
           source = CASE
                      WHEN $2 AND source = 'gravatar' THEN 'letter'
                      WHEN NOT $2 AND source IN ('letter', 'generated') AND gravatar_file_id IS NOT NULL THEN 'gravatar'
                      ELSE source
                    END,
           updated_at = now()
       WHERE user_id = $1"#,
    user_id.as_str(),
    dismissed,
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected())
}
