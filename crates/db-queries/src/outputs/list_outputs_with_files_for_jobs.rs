use ids::JobId;
use sqlx::PgExecutor;

use super::OutputWithFile;

/// Batch twin of [`list_outputs_with_files_for_job`] — one round-trip for a whole listing
/// page (callers group by `job_id` in memory).
pub async fn list_outputs_with_files_for_jobs<'e, E: PgExecutor<'e>>(
  exec: E,
  job_ids: &[JobId],
) -> sqlx::Result<Vec<OutputWithFile>> {
  let ids: Vec<String> = job_ids.iter().map(|j| j.as_str().to_string()).collect();
  sqlx::query_as!(
    OutputWithFile,
    r#"SELECT o.job_id AS "job_id: JobId", o.id AS "output_id: ids::OutputId", o.output_index, o.kind,
              f.id AS "file_id: ids::FileId", f.bucket_folder, f.bucket_hash,
              f.bucket_suffix, f.bucket_extension, f.mime, f.bytes
       FROM outputs o
       JOIN files f ON f.id = o.file_id
       WHERE o.job_id = ANY($1)
       ORDER BY o.job_id, o.output_index"#,
    &ids,
  )
  .fetch_all(exec)
  .await
}
