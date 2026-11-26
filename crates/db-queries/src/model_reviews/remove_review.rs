use ids::UserId;
use sqlx::PgExecutor;

use super::ReviewChange;

pub async fn remove_review<'e>(
    exec: impl PgExecutor<'e>,
    model: &str,
    user: &UserId,
) -> sqlx::Result<Option<ReviewChange>> {
    sqlx::query_as!(
        ReviewChange,
        r#"DELETE FROM model_reviews WHERE model = $1 AND user_id = $2
           RETURNING sid, NULL::bigint AS "text_revision?""#,
        model,
        user.as_str(),
    )
    .fetch_optional(exec)
    .await
}
