use enums::JobEventType;
use ids::{JobEventId, JobId};
use sqlx::PgExecutor;

pub async fn create_job_event<'e, E: PgExecutor<'e>>(
  exec: E,
  job_id: &JobId,
  event_type: JobEventType,
  data: Option<serde_json::Value>,
) -> sqlx::Result<()> {
  let id = JobEventId::generate();
  sqlx::query!(
    r#"INSERT INTO job_events (id, job_id, event_type, data) VALUES ($1, $2, $3, $4)"#,
    id.as_str(),
    job_id.as_str(),
    event_type.as_str(),
    data,
  )
  .execute(exec)
  .await?;
  Ok(())
}
