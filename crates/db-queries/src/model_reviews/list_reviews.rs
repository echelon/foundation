use sqlx::PgExecutor;

use super::PublicReview;

pub struct ReviewPage<'a> {
    pub models: &'a [String],
    pub before: Option<i64>,
    pub limit_per_model: i64,
}

pub async fn list_reviews<'e>(
    exec: impl PgExecutor<'e>,
    page: ReviewPage<'_>,
) -> sqlx::Result<Vec<PublicReview>> {
    sqlx::query_as!(
        PublicReview,
        r#"SELECT review.sid AS "sid!", review.text_revision AS "text_revision!", review.model AS "model!", review.rating AS "rating!",
                  review.review_text AS "review_text!", review.updated_at AS "updated_at!"
           FROM unnest($1::text[]) AS slug(model)
           CROSS JOIN LATERAL (
               SELECT r.sid, r.text_revision, r.model, r.rating, r.review_text, r.updated_at
               FROM model_reviews r JOIN users ON users.id = r.user_id
               WHERE r.model = slug.model AND r.review_text IS NOT NULL AND users.deleted_at IS NULL
                 AND r.sid < coalesce($2, 9223372036854775807)
               ORDER BY r.sid DESC LIMIT $3
           ) AS review
           ORDER BY review.model, review.sid DESC"#,
        page.models,
        page.before,
        page.limit_per_model,
    )
    .fetch_all(exec)
    .await
}
