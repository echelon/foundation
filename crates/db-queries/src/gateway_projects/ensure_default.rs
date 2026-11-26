use ids::AccountId;
use sqlx::PgExecutor;

pub async fn ensure_default<'e, E: PgExecutor<'e>>(
    exec: E,
    account: &AccountId,
) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO gateway_projects (id, account_id, name, is_default)
         SELECT 'proj_' || substr(a.id, 6), a.id, 'Default', true
         FROM accounts a WHERE a.id = $1 AND NOT EXISTS (
           SELECT 1 FROM gateway_projects p WHERE p.account_id = a.id AND p.is_default
         ) ON CONFLICT (account_id) WHERE is_default DO NOTHING",
        account.as_str(),
    )
    .execute(exec)
    .await?;
    Ok(())
}
