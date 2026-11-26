use sqlx::PgExecutor;

use super::Refresh;

pub async fn claim_refresh<'e>(
    exec: impl PgExecutor<'e>,
    models: &[String],
) -> sqlx::Result<Option<Refresh>> {
    sqlx::query_as!(
        Refresh,
        r#"UPDATE model_review_briefs AS brief SET refresh_after = now() + interval '5 minutes'
         FROM (
             SELECT model FROM model_review_briefs
             WHERE model = ANY($1) AND (revision > published_revision OR baseline IS NULL)
               AND refresh_after <= now()
             ORDER BY refresh_after, model
             LIMIT 1 FOR UPDATE SKIP LOCKED
         ) AS due
         WHERE brief.model = due.model
         RETURNING brief.model AS "model!", brief.revision AS "revision!",
                   brief.refresh_after AS "refresh_after!", brief.baseline, brief.community,
                   brief.review_sample AS "review_sample!""#,
        models,
    )
    .fetch_optional(exec)
    .await
}
