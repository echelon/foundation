use ids::UserId;
use sqlx::PgExecutor;

/// Set (or clear) the user's display name. The caller writes the org-visible
/// `display_name_changed` audit event in the same transaction (audit_logs.md §3) and refreshes
/// the avatar's person-letter monogram. Returns rows affected.
pub async fn update_display_name<'e, E: PgExecutor<'e>>(
  exec: E,
  user_id: &UserId,
  display_name: Option<&str>,
) -> sqlx::Result<u64> {
  let result = sqlx::query!(
    r#"UPDATE users SET display_name = $2, updated_at = now() WHERE id = $1"#,
    user_id.as_str(),
    display_name,
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected())
}
