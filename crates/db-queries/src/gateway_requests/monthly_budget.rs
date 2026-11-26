use chrono::{DateTime, Utc};
use ids::{AccountId, GatewayProjectId};
use serde::Serialize;
use sqlx::{PgConnection, PgExecutor};

use super::RequestScope;

#[derive(Serialize)]
pub struct MonthlyBudget {
    pub reserved_usd_nanos: String,
    pub limit_usd_nanos: Option<i64>,
    pub resets_at: DateTime<Utc>,
    #[serde(skip)]
    pub within_limit: bool,
}

// Each admission sees its own request and any commits from earlier lock holders.
pub async fn within_limits(
    conn: &mut PgConnection,
    scope: RequestScope<'_>,
    organization: Option<i64>,
    project: Option<i64>,
    client: Option<i64>,
) -> sqlx::Result<bool> {
    let client_ref = scope.client_ref;
    // Policy read locks keep the configured limits stable through admission. Every request
    // contributing to an enabled budget holds the same aggregate lock or a wider one.
    let lock = match (organization, project, client.zip(client_ref)) {
        (Some(_), _, _) => scope.account.as_str().to_owned(),
        (None, Some(_), _) => scope.project.as_str().to_owned(),
        (None, None, Some((_, reference))) => format!("{}:{reference}", scope.project),
        (None, None, None) => return Ok(true),
    };
    sqlx::query!(
        "SELECT pg_advisory_xact_lock(hashtextextended('gateway_monthly_budget:' || $1, 0))",
        lock
    )
    .execute(&mut *conn)
    .await?;
    for (target, reference, cap) in [
        (None, None, organization),
        (Some(scope.project), None, project),
        (Some(scope.project), client_ref, client),
    ] {
        if cap.is_some() {
            if !usage(&mut *conn, scope.account, target, reference, cap)
                .await?
                .within_limit
            {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

pub async fn usage<'e>(
    exec: impl PgExecutor<'e>,
    account: &AccountId,
    project: Option<&GatewayProjectId>,
    client_ref: Option<&str>,
    limit: Option<i64>,
) -> sqlx::Result<MonthlyBudget> {
    sqlx::query_as!(
        MonthlyBudget,
        r#"WITH calendar AS (
             SELECT date_trunc('month', transaction_timestamp() AT TIME ZONE 'UTC') AS start
           ), usage AS (
             SELECT COALESCE(SUM(GREATEST(r.budget_reserved_usd_nanos, r.estimated_usd_nanos)), 0) AS total
             FROM gateway_requests r CROSS JOIN calendar c
             WHERE r.account_id = $1 AND ($2::text IS NULL OR r.project_id = $2)
               AND r.created_at >= (c.start AT TIME ZONE 'UTC')
               AND r.created_at < ((c.start + interval '1 month') AT TIME ZONE 'UTC')
               AND (r.canceled_at IS NULL OR r.dispatched_at IS NOT NULL)
               AND ($4::text IS NULL OR r.client_ref = $4)
           )
           SELECT total::text AS "reserved_usd_nanos!",
                  $3::bigint AS "limit_usd_nanos?",
                  ((start + interval '1 month') AT TIME ZONE 'UTC') AS "resets_at!",
                  ($3::bigint IS NULL OR total <= $3::bigint) AS "within_limit!"
           FROM usage CROSS JOIN calendar"#,
        account.as_str(),
        project.map(GatewayProjectId::as_str),
        limit,
        client_ref,
    )
    .fetch_one(exec)
    .await
}
