use ids::{AccountId, GatewayProjectId, GatewayRequestId};
use sqlx::PgExecutor;

use super::RequestLocation;

pub async fn find_request_location<'e, E: PgExecutor<'e>>(
    exec: E,
    id: &GatewayRequestId,
) -> sqlx::Result<Option<RequestLocation>> {
    sqlx::query_as!(
        RequestLocation,
        r#"SELECT account_id AS "account_id: AccountId",
                  project_id AS "project_id: GatewayProjectId", client_ref
           FROM gateway_requests WHERE id = $1"#,
        id.as_str(),
    )
    .fetch_optional(exec)
    .await
}
