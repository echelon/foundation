use ids::{AccountId, GatewayProjectId};
use sqlx::PgExecutor;

pub struct ClientBudget {
    pub client_ref: String,
    pub revision: i32,
    pub limit_usd_nanos: Option<i64>,
}

pub async fn find<'e>(
    exec: impl PgExecutor<'e>,
    account: &AccountId,
    project: &GatewayProjectId,
    client_ref: &str,
) -> sqlx::Result<Option<ClientBudget>> {
    sqlx::query_as!(
        ClientBudget,
        "SELECT client_ref, revision, limit_usd_nanos FROM gateway_client_budgets
         WHERE account_id = $1 AND project_id = $2 AND client_ref = $3",
        account.as_str(), project.as_str(), client_ref,
    )
    .fetch_optional(exec)
    .await
}

pub async fn list<'e>(
    exec: impl PgExecutor<'e>,
    account: &AccountId,
    project: &GatewayProjectId,
    after: Option<&str>,
    limit: i64,
) -> sqlx::Result<Vec<ClientBudget>> {
    sqlx::query_as!(
        ClientBudget,
        "SELECT client_ref, revision, limit_usd_nanos FROM gateway_client_budgets
         WHERE account_id = $1 AND project_id = $2 AND limit_usd_nanos IS NOT NULL
           AND ($3::text IS NULL OR client_ref > $3)
         ORDER BY client_ref LIMIT $4",
        account.as_str(), project.as_str(), after, limit,
    )
    .fetch_all(exec)
    .await
}

/// The caller holds the organization policy write lock when changing a revision.
pub async fn save<'e>(
    exec: impl PgExecutor<'e>,
    account: &AccountId,
    project: &GatewayProjectId,
    budget: &ClientBudget,
) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO gateway_client_budgets (account_id, project_id, client_ref, revision, limit_usd_nanos)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (account_id, project_id, client_ref) DO UPDATE
         SET revision = excluded.revision, limit_usd_nanos = excluded.limit_usd_nanos",
        account.as_str(), project.as_str(), budget.client_ref, budget.revision, budget.limit_usd_nanos,
    )
    .execute(exec)
    .await?;
    Ok(())
}
