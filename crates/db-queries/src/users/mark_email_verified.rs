use ids::UserId;
use sqlx::PgExecutor;

/// Set `email_verified_at = now()` when a verification token is validated (accounts.md §2). Only
/// affects a currently-unverified user. Returns rows affected.
pub async fn mark_email_verified<'e, E: PgExecutor<'e>>(exec: E, user_id: &UserId) -> sqlx::Result<u64> {
  let result = sqlx::query!(
    r#"UPDATE users SET email_verified_at = now(), updated_at = now()
       WHERE id = $1 AND email_verified_at IS NULL"#,
    user_id.as_str(),
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected())
}
