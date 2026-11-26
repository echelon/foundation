use ids::GatewayRequestId;
use sqlx::PgExecutor;

use super::{Admission, RequestScope};

#[derive(Clone, Copy)]
pub struct AdmissionKey<'a> {
    pub scope: RequestScope<'a>,
    pub actor_id: &'a str,
    pub idempotency_digest: &'a str,
}

pub async fn find_admission<'e, E: PgExecutor<'e>>(
    exec: E,
    key: AdmissionKey<'_>,
) -> sqlx::Result<Option<Admission>> {
    sqlx::query_as!(
        Admission,
        r#"SELECT id AS "id: GatewayRequestId", intent_digest, trace_id
           FROM gateway_requests WHERE account_id = $1 AND project_id = $2 AND actor_id = $3 AND idempotency_digest = $4
             AND ($5::text IS NULL OR client_ref = $5)"#,
        key.scope.account.as_str(), key.scope.project.as_str(), key.actor_id, key.idempotency_digest, key.scope.client_ref,
    ).fetch_optional(exec).await
}
