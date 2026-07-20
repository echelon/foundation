use ids::UserId;
use sqlx::PgExecutor;

/// Bump `session_version` — the O(1) "log out all sessions" vector clock (auth.md §3).
pub async fn bump_session_version<'e, E: PgExecutor<'e>>(exec: E, user_id: &UserId) -> sqlx::Result<i32> {
  sqlx::query_scalar!(
    r#"UPDATE users SET session_version = session_version + 1, updated_at = now()
       WHERE id = $1 RETURNING session_version"#,
    user_id.as_str(),
  )
  .fetch_one(exec)
  .await
}
