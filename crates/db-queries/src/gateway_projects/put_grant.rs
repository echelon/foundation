use ids::{AccountId, AccountMemberId, GatewayProjectId};
use sqlx::PgExecutor;

pub struct ProjectGrant<'a> {
    pub account: &'a AccountId,
    pub project: &'a GatewayProjectId,
    pub membership: &'a AccountMemberId,
    pub role: &'a str,
    pub client_ref: Option<&'a str>,
}

pub async fn put_grant<'e, E: PgExecutor<'e>>(
    exec: E,
    grant: ProjectGrant<'_>,
) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO gateway_project_grants (account_id, project_id, membership_id, role, client_ref)
         VALUES ($1, $2, $3, $4, $5) ON CONFLICT (account_id, project_id, membership_id)
         DO UPDATE SET role = EXCLUDED.role, client_ref = EXCLUDED.client_ref",
        grant.account.as_str(), grant.project.as_str(), grant.membership.as_str(), grant.role, grant.client_ref,
    ).execute(exec).await?;
    Ok(())
}
