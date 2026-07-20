use ids::SsoConnectionId;
use sqlx::PgExecutor;

/// Bump `last_used_at` on a successful SSO login.
pub async fn touch_sso_last_used<'e, E: PgExecutor<'e>>(
  exec: E,
  id: &SsoConnectionId,
) -> sqlx::Result<()> {
  sqlx::query!(
    r#"UPDATE user_sso_connections SET last_used_at = now() WHERE id = $1"#,
    id.as_str(),
  )
  .execute(exec)
  .await?;
  Ok(())
}
