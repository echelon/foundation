use sqlx::PgExecutor;

use super::Brief;

pub async fn list_current<'e>(
    exec: impl PgExecutor<'e>,
    models: &[String],
) -> sqlx::Result<Vec<Brief>> {
    sqlx::query_as!(
        Brief,
        r#"SELECT model, rating_count, rating_total, written_reviews_sampled, strengths, weaknesses,
                  refreshed_at AS "refreshed_at!", baseline, community
           FROM model_review_briefs
           WHERE model = ANY($1) AND refreshed_at IS NOT NULL
           ORDER BY model"#,
        models,
    )
    .fetch_all(exec)
    .await
}
