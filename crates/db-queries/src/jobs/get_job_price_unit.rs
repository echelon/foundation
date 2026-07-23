use ids::JobId;
use sqlx::PgExecutor;

/// The job's stored billing unit (`per_image`, `per_video_second`, …) — kept
/// off the main `Job` row struct (billing internals, not API surface).
pub async fn get_job_price_unit<'e, E: PgExecutor<'e>>(exec: E, id: &JobId) -> sqlx::Result<Option<String>> {
  sqlx::query_scalar!("SELECT price_unit FROM jobs WHERE id = $1", id.as_str()).fetch_optional(exec).await
}
