use serde_json::Value;
use sqlx::PgExecutor;

pub async fn save_baseline<'e>(
    exec: impl PgExecutor<'e>,
    model: &str,
    baseline: &Value,
) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE model_review_briefs
         SET baseline = $2, refreshed_at = coalesce(refreshed_at, now())
         WHERE model = $1 AND baseline IS NULL",
        model,
        baseline,
    )
    .execute(exec)
    .await?;
    Ok(())
}
