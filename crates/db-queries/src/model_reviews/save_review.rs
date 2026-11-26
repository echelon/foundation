use ids::UserId;
use sqlx::PgExecutor;

use super::ReviewChange;

pub struct ReviewInput<'a> {
    pub model: &'a str,
    pub user: &'a UserId,
    pub rating: i16,
    pub review_text: Option<&'a str>,
}

pub async fn save_review<'e>(
    exec: impl PgExecutor<'e>,
    input: ReviewInput<'_>,
) -> sqlx::Result<Option<ReviewChange>> {
    sqlx::query_as!(
        ReviewChange,
        r#"INSERT INTO model_reviews (model, user_id, rating, review_text)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (model, user_id) DO UPDATE
         SET rating = EXCLUDED.rating, review_text = EXCLUDED.review_text,
             updated_at = now(),
             text_revision = model_reviews.text_revision
                 + (model_reviews.review_text IS DISTINCT FROM EXCLUDED.review_text)::integer
         WHERE (model_reviews.rating, model_reviews.review_text)
               IS DISTINCT FROM (EXCLUDED.rating, EXCLUDED.review_text)
         RETURNING sid, text_revision AS "text_revision?""#,
        input.model,
        input.user.as_str(),
        input.rating,
        input.review_text,
    )
    .fetch_optional(exec)
    .await
}
