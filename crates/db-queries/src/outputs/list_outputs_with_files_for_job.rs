use ids::{FileId, JobId, OutputId};
use sqlx::PgExecutor;

/// An output joined with its file's bucket-path pieces — everything needed to DERIVE the
/// CDN URL (files.md: URLs are never stored).
#[derive(Debug, Clone)]
pub struct OutputWithFile {
  pub job_id: JobId,
  pub output_id: OutputId,
  pub output_index: i32,
  pub kind: String,
  pub file_id: FileId,
  pub bucket_folder: String,
  pub bucket_hash: String,
  pub bucket_suffix: Option<String>,
  pub bucket_extension: Option<String>,
  pub mime: String,
  pub bytes: i64,
}

pub async fn list_outputs_with_files_for_job<'e, E: PgExecutor<'e>>(
  exec: E,
  job_id: &JobId,
) -> sqlx::Result<Vec<OutputWithFile>> {
  sqlx::query_as!(
    OutputWithFile,
    r#"SELECT o.job_id AS "job_id: JobId", o.id AS "output_id: OutputId", o.output_index, o.kind,
              f.id AS "file_id: FileId", f.bucket_folder, f.bucket_hash,
              f.bucket_suffix, f.bucket_extension, f.mime, f.bytes
       FROM outputs o
       JOIN files f ON f.id = o.file_id
       WHERE o.job_id = $1
       ORDER BY o.output_index"#,
    job_id.as_str(),
  )
  .fetch_all(exec)
  .await
}
