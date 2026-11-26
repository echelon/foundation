use ids::{AccountId, AccountMemberId, GatewayProjectId};
use sqlx::PgExecutor;

pub async fn delete_grant<'e, E: PgExecutor<'e>>(
    exec: E,
    account: &AccountId,
    project: &GatewayProjectId,
    membership: &AccountMemberId,
) -> sqlx::Result<bool> {
    let rows = sqlx::query!(
        "DELETE FROM gateway_project_grants WHERE account_id = $1 AND project_id = $2 AND membership_id = $3",
        account.as_str(), project.as_str(), membership.as_str(),
    ).execute(exec).await?.rows_affected();
    Ok(rows == 1)
}
