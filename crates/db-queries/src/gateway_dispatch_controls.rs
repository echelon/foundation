use ids::AccountId;
use sqlx::{PgExecutor, Postgres, Transaction};

#[derive(Default)]
pub struct DispatchControl {
    pub revision: i32,
    pub pause_all: bool,
    pub paused_providers: Vec<String>,
}

pub async fn read<'e>(
    exec: impl PgExecutor<'e>,
    account: &AccountId,
) -> sqlx::Result<DispatchControl> {
    Ok(sqlx::query_as!(
        DispatchControl,
        "SELECT revision, pause_all, paused_providers FROM gateway_dispatch_controls WHERE account_id = $1",
        account.as_str(),
    )
    .fetch_optional(exec)
    .await?
    .unwrap_or_default())
}

pub async fn save(
    tx: &mut Transaction<'_, Postgres>,
    account: &AccountId,
    expected_revision: i32,
    pause_all: bool,
    paused_providers: &[String],
) -> sqlx::Result<Option<DispatchControl>> {
    crate::gateway_requests::lock_submission_lanes(tx).await?;
    sqlx::query_as!(
        DispatchControl,
        "INSERT INTO gateway_dispatch_controls AS current (account_id, revision, pause_all, paused_providers)
         SELECT $1, $2::integer + 1, $3, $4
         WHERE $2 = 0 OR EXISTS (
             SELECT 1 FROM gateway_dispatch_controls WHERE account_id = $1 AND revision = $2
         )
         ON CONFLICT (account_id) DO UPDATE SET
           revision = excluded.revision, pause_all = excluded.pause_all,
           paused_providers = excluded.paused_providers
         WHERE current.revision = $2
         RETURNING revision, pause_all, paused_providers",
        account.as_str(),
        expected_revision,
        pause_all,
        paused_providers,
    )
    .fetch_optional(&mut **tx)
    .await
}
