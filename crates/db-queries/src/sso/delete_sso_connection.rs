use ids::{SsoConnectionId, UserId};
use sqlx::PgExecutor;

/// Disconnect an SSO connection — scoped by user so a foreign connection id can never be
/// touched. Returns rows affected (0 = unknown/foreign id).
pub async fn delete_sso_connection<'e, E: PgExecutor<'e>>(
  exec: E,
  user_id: &UserId,
  id: &SsoConnectionId,
) -> sqlx::Result<u64> {
  let result = sqlx::query!(
    r#"DELETE FROM user_sso_connections WHERE id = $1 AND user_id = $2"#,
    id.as_str(),
    user_id.as_str(),
  )
  .execute(exec)
  .await?;
  Ok(result.rows_affected())
}
