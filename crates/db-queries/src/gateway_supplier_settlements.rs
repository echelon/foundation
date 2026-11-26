use chrono::{DateTime, Utc};
use ids::GatewayRequestId;
use sqlx::PgExecutor;

use crate::gateway_requests::RequestScope;

pub struct SettlementTarget {
    pub eligible: bool,
    pub provider: String,
    pub estimated_usd_nanos: i64,
}

pub async fn target<'e>(
    exec: impl PgExecutor<'e>,
    scope: RequestScope<'_>,
    request: &GatewayRequestId,
) -> sqlx::Result<Option<SettlementTarget>> {
    sqlx::query_as!(
        SettlementTarget,
        r#"SELECT (adapter <> 'mock' AND dispatched_at IS NOT NULL
                    AND num_nonnulls(succeeded_at, failed_at, canceled_at, uncertain_at) <> 0) AS "eligible!",
                  provider, estimated_usd_nanos
           FROM gateway_requests WHERE id = $1 AND account_id = $2 AND project_id = $3
           FOR UPDATE"#,
        request.as_str(), scope.account.as_str(), scope.project.as_str(),
    ).fetch_optional(exec).await
}

pub struct Posting {
    pub revision: i32,
    pub amount_usd_nanos: i64,
    pub delta_usd_nanos: i64,
    pub evidence_ref: String,
    pub recorded_at: DateTime<Utc>,
}

pub async fn history<'e>(
    exec: impl PgExecutor<'e>,
    request: &GatewayRequestId,
) -> sqlx::Result<Vec<Posting>> {
    sqlx::query_as!(
        Posting,
        "SELECT revision, amount_usd_nanos, delta_usd_nanos, evidence_ref, recorded_at
         FROM gateway_supplier_settlements WHERE request_id = $1 ORDER BY revision DESC LIMIT 25",
        request.as_str(),
    )
    .fetch_all(exec)
    .await
}

pub async fn replay<'e>(
    exec: impl PgExecutor<'e>,
    request: &GatewayRequestId,
    key: &str,
) -> sqlx::Result<Option<Posting>> {
    sqlx::query_as!(
        Posting,
        "SELECT revision, amount_usd_nanos, delta_usd_nanos, evidence_ref, recorded_at
         FROM gateway_supplier_settlements WHERE request_id = $1 AND idempotency_digest = $2",
        request.as_str(),
        key,
    )
    .fetch_optional(exec)
    .await
}

pub async fn latest_revision<'e>(
    exec: impl PgExecutor<'e>,
    request: &GatewayRequestId,
) -> sqlx::Result<Option<i32>> {
    sqlx::query_scalar!(
        "SELECT revision FROM gateway_supplier_settlements WHERE request_id = $1
         ORDER BY revision DESC LIMIT 1",
        request.as_str(),
    )
    .fetch_optional(exec)
    .await
}

pub struct NewPosting<'a> {
    pub request: &'a GatewayRequestId,
    pub revision: i32,
    pub amount_usd_nanos: i64,
    pub evidence_ref: &'a str,
    pub idempotency_digest: &'a str,
}

pub async fn append<'e>(
    exec: impl PgExecutor<'e>,
    posting: NewPosting<'_>,
) -> sqlx::Result<Posting> {
    sqlx::query_as!(
        Posting,
        "INSERT INTO gateway_supplier_settlements
           (request_id, revision, amount_usd_nanos, evidence_ref, idempotency_digest)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING revision, amount_usd_nanos, delta_usd_nanos, evidence_ref, recorded_at",
        posting.request.as_str(),
        posting.revision,
        posting.amount_usd_nanos,
        posting.evidence_ref,
        posting.idempotency_digest,
    )
    .fetch_one(exec)
    .await
}
