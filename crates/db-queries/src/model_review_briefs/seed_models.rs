use sqlx::PgExecutor;

pub async fn seed_models<'e>(exec: impl PgExecutor<'e>, models: &[String]) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO model_review_briefs (model)
         SELECT unnest($1::text[])
         ON CONFLICT (model) DO NOTHING",
        models,
    )
    .execute(exec)
    .await?;
    Ok(())
}
