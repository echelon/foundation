use ids::AccountId;
use sqlx::PgExecutor;

pub async fn lock_for_update<'e>(
    exec: impl PgExecutor<'e>,
    account: &AccountId,
) -> sqlx::Result<()> {
    sqlx::query_scalar!(
        "SELECT id FROM accounts WHERE id = $1 FOR NO KEY UPDATE",
        account.as_str(),
    )
    .fetch_one(exec)
    .await?;
    Ok(())
}
