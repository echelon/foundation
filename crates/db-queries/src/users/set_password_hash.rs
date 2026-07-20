use ids::UserId;
use sqlx::PgExecutor;

/// Set/replace the password hash and bump `password_version` (a record, not the logout clock —
/// auth.md §3). Returns rows affected.
pub async fn set_password_hash<'e, E: PgExecutor<'e>>(exec: E, user_id: &UserId, hash: &str) -> sqlx::Result<u64> {
  let r = sqlx::query!(
    r#"UPDATE users
       SET password_hash = $2, password_version = password_version + 1, updated_at = now()
       WHERE id = $1"#,
    user_id.as_str(),
    hash,
  )
  .execute(exec)
  .await?;
  Ok(r.rows_affected())
}
