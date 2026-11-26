use sqlx::PgExecutor;

use super::ModelRating;

pub async fn model_ratings<'e>(
    exec: impl PgExecutor<'e>,
    models: &[String],
) -> sqlx::Result<Vec<ModelRating>> {
    sqlx::query_as!(
        ModelRating,
        r#"SELECT model, count(*) AS "rating_count!", sum(rating) AS "rating_total!"
           FROM model_reviews JOIN users ON users.id = model_reviews.user_id
           WHERE model = ANY($1) AND users.deleted_at IS NULL
           GROUP BY model"#,
        models,
    )
    .fetch_all(exec)
    .await
}
