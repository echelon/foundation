use ids::AccountId;
use sqlx::PgExecutor;

// A shared lock orders admission against a policy update on the same account.
pub async fn lock_for_admission<'e>(
    exec: impl PgExecutor<'e>,
    account: &AccountId,
) -> sqlx::Result<()> {
    sqlx::query_scalar!(
        "SELECT id FROM accounts WHERE id = $1 FOR SHARE",
        account.as_str()
    )
    .fetch_one(exec)
    .await?;
    Ok(())
}
