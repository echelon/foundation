use ids::UserId;
use sqlx::PgExecutor;

/// Refresh the person-name monogram letters after a display-name change (accounts.md §2 — the
/// letter avatar prefers person-name letters over email letters when a name is known).
/// Returns rows affected.
pub async fn set_person_letters<'e, E: PgExecutor<'e>>(
  exec: E,
  user_id: &UserId,
  first: Option<&str>,
  last: Option<&str>,
) -> sqlx::Result<u64> {
  let result = sqlx::query!(
    r#"UPDATE user_avatars
       SET person_name_first_letter = $2, person_name_last_letter = $3, updated_at = now()
       WHERE user_id = $1"#,
    user_id.as_str(),
    first,
    last,
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected())
}
