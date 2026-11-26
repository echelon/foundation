use ids::{AccountId, AccountMemberId, GatewayProjectId};
use sqlx::PgExecutor;

use super::ProjectAccess;

#[derive(Default)]
pub struct ProjectFilter<'a> {
    pub project: Option<&'a GatewayProjectId>,
    pub membership: Option<&'a AccountMemberId>,
    pub admin: bool,
    pub default_only: bool,
}

pub async fn list_projects<'e, E: PgExecutor<'e>>(
    exec: E,
    account: &AccountId,
    filter: ProjectFilter<'_>,
) -> sqlx::Result<Vec<ProjectAccess>> {
    sqlx::query_as!(
        ProjectAccess,
        r#"SELECT p.id AS "id: GatewayProjectId", p.name, p.is_default,
             CASE WHEN $4 THEN 'admin' WHEN p.is_default THEN 'developer' ELSE g.role END AS "role!",
             CASE WHEN $4 OR p.is_default THEN NULL ELSE g.client_ref END AS "client_ref?"
           FROM gateway_projects p LEFT JOIN gateway_project_grants g
             ON g.account_id = p.account_id AND g.project_id = p.id AND g.membership_id = $3
           WHERE p.account_id = $1 AND ($2::text IS NULL OR p.id = $2)
             AND (NOT $5 OR p.is_default) AND ($4 OR p.is_default OR g.role IS NOT NULL)
           ORDER BY p.is_default DESC, p.name, p.id"#,
        account.as_str(), filter.project.map(GatewayProjectId::as_str),
        filter.membership.map(AccountMemberId::as_str), filter.admin, filter.default_only,
    ).fetch_all(exec).await
}
