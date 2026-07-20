use ids::SessionId;
use sqlx::PgExecutor;

/// Idempotent logout: sets `revoked_at` if not already set. Returns whether a row was updated.
pub async fn revoke_session<'e, E: PgExecutor<'e>>(exec: E, id: &SessionId) -> sqlx::Result<bool> {
  let result = sqlx::query!(
    r#"UPDATE sessions SET revoked_at = now()
       WHERE id = $1 AND revoked_at IS NULL"#,
    id.as_str(),
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected() > 0)
}
