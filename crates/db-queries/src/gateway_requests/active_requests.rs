use std::collections::BTreeMap;

use ids::{AccountId, GatewayProjectId};
use serde::Serialize;
use sqlx::PgExecutor;

#[derive(Serialize)]
pub struct ActiveRequests {
    pub total: i64,
    pub by_provider: BTreeMap<String, i64>,
}

pub async fn lock<'e>(exec: impl PgExecutor<'e>, account: &AccountId) -> sqlx::Result<()> {
    sqlx::query!(
        "SELECT pg_advisory_xact_lock(hashtextextended('gateway_active_requests:' || $1, 0))",
        account.as_str(),
    )
    .execute(exec)
    .await?;
    Ok(())
}

pub async fn usage<'e>(
    exec: impl PgExecutor<'e>,
    account: &AccountId,
    project: Option<&GatewayProjectId>,
) -> sqlx::Result<ActiveRequests> {
    let by_provider: BTreeMap<_, _> = sqlx::query!(
        r#"SELECT provider, count(*) AS "active!"
           FROM gateway_requests
           WHERE account_id = $1 AND ($2::text IS NULL OR project_id = $2)
             AND num_nonnulls(succeeded_at, failed_at, canceled_at) = 0
           GROUP BY provider"#,
        account.as_str(),
        project.map(GatewayProjectId::as_str),
    )
    .fetch_all(exec)
    .await?
    .into_iter()
    .map(|row| (row.provider, row.active))
    .collect();
    Ok(ActiveRequests {
        total: by_provider.values().sum(),
        by_provider,
    })
}
