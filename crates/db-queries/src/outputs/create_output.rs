use ids::{FileId, JobId, OutputId};
use sqlx::PgExecutor;

use super::Output;

#[derive(Debug)]
pub struct NewOutput<'a> {
  pub job_id: &'a JobId,
  pub file_id: &'a FileId,
  pub output_index: i32,
  pub kind: &'a str,
  pub width: Option<i32>,
  pub height: Option<i32>,
}

pub async fn create_output<'e, E: PgExecutor<'e>>(exec: E, new: NewOutput<'_>) -> sqlx::Result<Output> {
  let id = OutputId::generate();
  sqlx::query_as!(
    Output,
    r#"INSERT INTO outputs (id, job_id, file_id, output_index, kind, width, height)
       VALUES ($1, $2, $3, $4, $5, $6, $7)
       RETURNING id AS "id: OutputId", job_id AS "job_id: JobId", file_id AS "file_id?: FileId",
                 output_index, kind, width, height, created_at"#,
    id.as_str(),
    new.job_id.as_str(),
    new.file_id.as_str(),
    new.output_index,
    new.kind,
    new.width,
    new.height,
  )
  .fetch_one(exec)
  .await
}
