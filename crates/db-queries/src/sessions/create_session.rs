use chrono::{DateTime, Utc};
use ids::{SessionId, UserId};
use sqlx::PgExecutor;

use super::Session;

#[derive(Debug, Clone)]
pub struct NewSession<'a> {
  pub user_id: &'a UserId,
  pub expires_at: DateTime<Utc>,
  pub ip: Option<&'a str>,
  pub user_agent: Option<&'a str>,
  pub device: Option<&'a str>,
  /// Set only for staff impersonation (auth.md §6).
  pub impersonator_user_id: Option<&'a UserId>,
}

pub async fn create_session<'e, E: PgExecutor<'e>>(exec: E, new: NewSession<'_>) -> sqlx::Result<Session> {
  let id = SessionId::generate();
  let impersonator = new.impersonator_user_id.map(|i| i.as_str());
  sqlx::query_as!(
    Session,
    r#"INSERT INTO sessions (id, user_id, impersonator_user_id, ip, user_agent, device, expires_at)
       VALUES ($1, $2, $3, ($4::text)::inet, $5, $6, $7)
       RETURNING id AS "id: SessionId", user_id AS "user_id: UserId",
                 impersonator_user_id AS "impersonator_user_id?: UserId",
                 ip::text AS "ip?: String", user_agent, device,
                 created_at, expires_at, revoked_at"#,
    id.as_str(),
    new.user_id.as_str(),
    impersonator,
    new.ip,
    new.user_agent,
    new.device,
    new.expires_at,
  )
  .fetch_one(exec)
  .await
}
