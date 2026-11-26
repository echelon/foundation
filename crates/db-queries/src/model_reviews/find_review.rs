use ids::UserId;
use sqlx::PgExecutor;

use super::Review;

pub async fn find_review<'e>(
    exec: impl PgExecutor<'e>,
    model: &str,
    user: &UserId,
) -> sqlx::Result<Option<Review>> {
    sqlx::query_as!(
        Review,
        "SELECT rating, review_text FROM model_reviews WHERE model = $1 AND user_id = $2",
        model,
        user.as_str(),
    )
    .fetch_optional(exec)
    .await
}
