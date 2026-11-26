use sqlx::PgExecutor;

use super::RequestScope;

pub struct RouteOutcome {
    pub offer_id: String,
    pub samples: i64,
    pub succeeded: i64,
    pub unconfirmed: i64,
    pub median_ms: Option<i64>,
    pub settled_samples: i64,
    pub settled_usd_nanos: Option<String>,
}

pub async fn recent_outcomes<'e>(
    exec: impl PgExecutor<'e>,
    scope: RequestScope<'_>,
    workload: &serde_json::Value,
    offers: &serde_json::Value,
    window_days: i32,
    sample_limit: i64,
    include_cost: bool,
) -> sqlx::Result<Vec<RouteOutcome>> {
    sqlx::query_as!(
        RouteOutcome,
        r#"WITH bounds AS (
             SELECT CASE WHEN $8::boolean THEN date_trunc('day', statement_timestamp(), 'UTC')
                         ELSE statement_timestamp() END AS until
           ), recent AS MATERIALIZED (
             SELECT r.id, r.sid, r.quote, r.offer_id, r.succeeded_at, r.uncertain_at,
                    r.failed_at, r.adapter, r.estimated_usd_nanos
             FROM gateway_requests r CROSS JOIN bounds w
             WHERE r.account_id = $1 AND r.project_id = $2
               AND (r.client_ref = $3::text OR ($3 IS NULL AND r.client_ref IS NULL))
               AND r.created_at >= w.until - $6::integer * interval '24 hours'
               AND (NOT $8::boolean OR r.created_at < w.until)
             ORDER BY r.sid DESC LIMIT 2000
           ), observed AS MATERIALIZED (
             SELECT r.id, r.sid, r.quote, r.offer_id AS original_offer_id, r.estimated_usd_nanos,
                    coalesce(p.route_retries->-1->'decision'->>'next_offer_id', r.offer_id) AS offer_id,
                    p.binding_digest, p.upstream_model, p.accepted_at, r.succeeded_at, r.uncertain_at
             FROM recent r JOIN gateway_provider_runs p ON p.request_id = r.id CROSS JOIN bounds w
             WHERE (NOT $8::boolean OR coalesce(r.succeeded_at, r.uncertain_at, r.failed_at) < w.until)
               AND r.adapter IN ('fal_queue', 'kinovi_queue', 'connector_queue')
               AND r.quote #> '{input,workload}' = $4::jsonb
               AND p.accepted_at IS NOT NULL
               AND (r.succeeded_at IS NOT NULL OR r.uncertain_at IS NOT NULL
                    OR (r.failed_at IS NOT NULL AND p.error_code = 'provider_generation_failed'))
           )
           SELECT offer.id AS "offer_id!", stats.samples AS "samples!",
                  stats.succeeded AS "succeeded!", stats.unconfirmed AS "unconfirmed!", stats.median_ms,
                  stats.settled_samples AS "settled_samples!", stats.settled_usd_nanos
           FROM jsonb_to_recordset($5::jsonb) AS offer (
               id text, binding_digest text, upstream_model text,
               applied_aspect_ratio text, output_size jsonb, estimated_usd_nanos bigint
           )
           CROSS JOIN LATERAL (
             SELECT count(*) AS samples, count(sample.succeeded_at) AS succeeded,
                    count(sample.uncertain_at) AS unconfirmed,
                    round(percentile_cont(0.5) WITHIN GROUP (
                        ORDER BY greatest(0, extract(epoch FROM sample.succeeded_at - sample.accepted_at) * 1000)
                    ) FILTER (WHERE sample.succeeded_at IS NOT NULL))::bigint AS median_ms,
                    count(settled.amount_usd_nanos) AS settled_samples,
                    sum(settled.amount_usd_nanos)::text AS settled_usd_nanos
             FROM (
               SELECT o.id, o.succeeded_at, o.uncertain_at, o.accepted_at FROM observed o
               WHERE o.offer_id = offer.id AND o.binding_digest = offer.binding_digest
                 AND o.upstream_model = offer.upstream_model
                 AND (NOT $8::boolean OR (o.original_offer_id = o.offer_id
                         AND o.estimated_usd_nanos = offer.estimated_usd_nanos))
                 AND (o.quote #>> '{candidates,0,applied_aspect_ratio}') IS NOT DISTINCT FROM offer.applied_aspect_ratio
                 AND nullif(o.quote #> '{candidates,0,output_size}', 'null'::jsonb)
                     IS NOT DISTINCT FROM offer.output_size
               ORDER BY o.sid DESC LIMIT $7
             ) sample
             LEFT JOIN LATERAL (
               SELECT s.amount_usd_nanos FROM gateway_supplier_settlements s
               WHERE $8::boolean AND s.request_id = sample.id
               ORDER BY s.revision DESC LIMIT 1
             ) settled ON true
           ) stats"#,
        scope.account.as_str(),
        scope.project.as_str(),
        scope.client_ref,
        workload,
        offers,
        window_days,
        sample_limit,
        include_cost,
    )
    .fetch_all(exec)
    .await
}
