use ids::SessionId;
use sqlx::PgExecutor;

use super::Session;

/// Looked up on every authenticated request; the caller checks `revoked_at` / `expires_at`.
pub async fn find_session_by_id<'e, E: PgExecutor<'e>>(
  exec: E,
  id: &SessionId,
) -> sqlx::Result<Option<Session>> {
  sqlx::query_as!(
    Session,
    r#"SELECT id AS "id: SessionId", user_id AS "user_id: ids::UserId",
              impersonator_user_id AS "impersonator_user_id?: ids::UserId",
              created_at, expires_at, revoked_at
       FROM sessions
       WHERE id = $1"#,
    id.as_str(),
  )
  .fetch_optional(exec)
  .await
}
