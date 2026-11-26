use serde_json::Value;
use sqlx::PgExecutor;

use super::Refresh;

pub struct BriefUpdate<'a> {
    pub refresh: &'a Refresh,
    pub rating_count: i64,
    pub rating_total: i64,
    pub written_reviews_sampled: i32,
    pub community: &'a Value,
    pub review_sample: &'a Value,
    pub reviews_processed: bool,
}

pub async fn publish<'e>(exec: impl PgExecutor<'e>, brief: BriefUpdate<'_>) -> sqlx::Result<bool> {
    let result = sqlx::query!(
        "UPDATE model_review_briefs SET
            published_revision = CASE WHEN $9 THEN revision ELSE published_revision END,
            refreshed_at = now(),
            refresh_after = CASE WHEN $9 AND baseline IS NULL THEN now() + interval '1 hour'
                                 ELSE refresh_after END,
            rating_count = $4, rating_total = $5, written_reviews_sampled = $6,
            community = $7, review_sample = $8
         WHERE model = $1 AND revision = $2 AND refresh_after = $3 AND refresh_after > now()",
        brief.refresh.model.as_str(),
        brief.refresh.revision,
        brief.refresh.refresh_after,
        brief.rating_count,
        brief.rating_total,
        brief.written_reviews_sampled,
        brief.community,
        brief.review_sample,
        brief.reviews_processed,
    )
    .execute(exec)
    .await?;
    Ok(result.rows_affected() == 1)
}
