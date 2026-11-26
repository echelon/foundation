use sqlx::PgExecutor;

use crate::model_reviews::ReviewChange;

pub async fn mark_dirty<'e>(
    exec: impl PgExecutor<'e>,
    model: &str,
    change: ReviewChange,
) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO model_review_briefs (model, refresh_after)
         VALUES ($1, now() + interval '30 seconds')
         ON CONFLICT (model) DO UPDATE SET
             revision = model_review_briefs.revision + 1,
             refresh_after = CASE
                 WHEN model_review_briefs.revision = model_review_briefs.published_revision
                     THEN EXCLUDED.refresh_after
                 ELSE least(model_review_briefs.refresh_after, EXCLUDED.refresh_after)
             END,
             community = (
                 SELECT coalesce(jsonb_agg(jsonb_set(topic, '{evidence}', evidence.ids)
                                          ORDER BY position), '[]'::jsonb)
                 FROM jsonb_array_elements(model_review_briefs.community)
                      WITH ORDINALITY AS topics(topic, position)
                 CROSS JOIN LATERAL (
                     SELECT coalesce(jsonb_agg(item ORDER BY evidence_position), '[]'::jsonb) AS ids
                     FROM jsonb_array_elements(topic->'evidence')
                          WITH ORDINALITY AS refs(item, evidence_position)
                     WHERE (item->>'sid')::bigint <> $2 OR (item->>'text_revision')::bigint = $3
                 ) AS evidence
                 WHERE jsonb_array_length(evidence.ids) >= 3
             )",
        model,
        change.sid,
        change.text_revision,
    )
    .execute(exec)
    .await?;
    Ok(())
}
