use sqlx::PgExecutor;

pub async fn read_snapshot<'e, E: PgExecutor<'e>>(exec: E) -> sqlx::Result<()> {
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(exec)
        .await?;
    Ok(())
}
