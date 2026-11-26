use sqlx::PgExecutor;

pub struct UsageFilter<'a> {
    pub models: &'a [String],
    pub aliases: &'a [String],
    pub minimum_accounts: i64,
}

/// Return only models that meet the gate. Neither organization IDs nor activity counts leave storage.
pub async fn used_models<'e>(
    exec: impl PgExecutor<'e>,
    filter: UsageFilter<'_>,
) -> sqlx::Result<Vec<String>> {
    sqlx::query_scalar!(
        r#"SELECT target.model AS "model!"
           FROM unnest($1::text[], $2::text[]) AS target(model, alias)
           WHERE (SELECT count(*) FROM (
               SELECT DISTINCT account_id FROM gateway_requests
               WHERE model IN (target.model, target.alias)
                 AND succeeded_at >= statement_timestamp() - interval '30 days'
                 AND succeeded_at <= statement_timestamp()
                 AND adapter <> 'mock'
               LIMIT $3
           ) AS active_accounts) = $3"#,
        filter.models,
        filter.aliases,
        filter.minimum_accounts,
    )
    .fetch_all(exec)
    .await
}
