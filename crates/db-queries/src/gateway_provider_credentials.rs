use chrono::{DateTime, Utc};
use ids::{AccountId, GatewayProjectId};
use sqlx::{FromRow, PgConnection, PgPool};

use crate::gateway_requests::RequestScope;

pub struct Scope<'a> {
    pub account: &'a AccountId,
    pub project: Option<&'a GatewayProjectId>,
    pub binding: &'a str,
    pub adapter: &'a str,
}

impl Scope<'_> {
    pub fn id(&self) -> &str {
        self.project.map_or(self.account.as_str(), GatewayProjectId::as_str)
    }
}

#[derive(FromRow)]
pub struct Version {
    pub version: i32,
    pub binding_digest: String,
    pub created_at: DateTime<Utc>,
    pub retired_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub has_key: bool,
    pub inherits_organization: bool,
}

pub async fn versions(conn: &mut PgConnection, scope: Scope<'_>, limit: i64) -> sqlx::Result<Vec<Version>> {
    sqlx::query_as(
        "SELECT version, binding_digest, created_at, retired_at, revoked_at,
                encrypted_key IS NOT NULL AS has_key, inherits_organization
         FROM gateway_provider_credentials
         WHERE account_id = $1 AND scope_id = $2 AND adapter = $4
           AND binding_slot = $3
         ORDER BY version DESC LIMIT $5",
    ).bind(scope.account.as_str()).bind(scope.id()).bind(scope.binding).bind(scope.adapter).bind(limit)
        .fetch_all(conn).await
}

#[derive(Clone, FromRow)]
pub struct Effective {
    pub adapter: String,
    pub binding: String,
    pub binding_digest: String,
    pub organization: bool,
    pub usable: bool,
    pub version: i32,
}

pub async fn effective(conn: &mut PgConnection, scope: RequestScope<'_>, bindings: &[(String, String)]) -> sqlx::Result<Vec<Effective>> {
    let (adapters, names): (Vec<_>, Vec<_>) = bindings.iter().cloned().unzip();
    sqlx::query_as(
        "WITH latest AS (
           SELECT DISTINCT ON (c.adapter, c.binding_slot, c.scope_id) c.*
           FROM gateway_provider_credentials c
           JOIN unnest($3::text[], $4::text[]) AS wanted(adapter, slot)
             ON c.adapter = wanted.adapter AND c.binding_slot = wanted.slot
           WHERE c.account_id = $1 AND c.scope_id IN ($1, $2)
           ORDER BY c.adapter, c.binding_slot, c.scope_id, c.version DESC
         )
         SELECT DISTINCT ON (adapter, binding_slot) adapter, binding_slot AS binding, binding_digest, version,
           scope_id = $1 AS organization,
           encrypted_key IS NOT NULL AND revoked_at IS NULL AND retired_at IS NULL AS usable
         FROM latest WHERE NOT inherits_organization
         ORDER BY adapter, binding_slot, (scope_id = $2) DESC",
    ).bind(scope.account.as_str()).bind(scope.project.as_str()).bind(adapters).bind(names)
        .fetch_all(conn).await
}

pub async fn save(conn: &mut PgConnection, scope: Scope<'_>, version: i32, digest: &str, key: Option<&[u8]>) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE gateway_provider_credentials SET retired_at = clock_timestamp()
         WHERE account_id = $1 AND scope_id = $2 AND adapter = $4
           AND binding_slot = $3 AND retired_at IS NULL AND revoked_at IS NULL",
    ).bind(scope.account.as_str()).bind(scope.id()).bind(scope.binding).bind(scope.adapter).execute(&mut *conn).await?;
    sqlx::query(
        "INSERT INTO gateway_provider_credentials
            (account_id, project_id, adapter, binding_slot, version, binding_digest, encrypted_key, inherits_organization)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $7::bytea IS NULL)",
    ).bind(scope.account.as_str()).bind(scope.project.map(GatewayProjectId::as_str))
        .bind(scope.adapter).bind(scope.binding).bind(version).bind(digest).bind(key).execute(conn).await?;
    Ok(())
}

pub async fn revoke(conn: &mut PgConnection, scope: Scope<'_>, version: i32) -> sqlx::Result<bool> {
    let result = sqlx::query(
        "UPDATE gateway_provider_credentials SET revoked_at = clock_timestamp(), encrypted_key = NULL
         WHERE account_id = $1 AND scope_id = $2 AND adapter = $4
           AND binding_slot = $3 AND version = $5 AND revoked_at IS NULL AND NOT inherits_organization",
    ).bind(scope.account.as_str()).bind(scope.id()).bind(scope.binding).bind(scope.adapter).bind(version).execute(conn).await?;
    Ok(result.rows_affected() == 1)
}

#[derive(FromRow)]
pub struct Pinned {
    pub adapter: String,
    pub account_id: String,
    pub scope_id: String,
    pub binding_slot: String,
    pub version: i32,
    pub encrypted_key: Vec<u8>,
}

pub async fn pinned(pool: &PgPool, request: &str, binding: &str) -> sqlx::Result<Option<Pinned>> {
    sqlx::query_as(
        "SELECT c.adapter, c.account_id, c.scope_id, c.binding_slot, c.version, c.encrypted_key
         FROM gateway_requests r
         JOIN gateway_provider_credentials c
           ON (c.account_id, c.scope_id, c.adapter, c.binding_slot, c.version) =
              (r.account_id, r.provider_credential_scope_id, r.adapter,
               r.provider_credential_binding_slot, r.provider_credential_version)
         WHERE r.id = $1 AND c.adapter IN ('connector_queue', 'openai_compatible_chat') AND c.binding_digest = $2
           AND c.encrypted_key IS NOT NULL AND c.revoked_at IS NULL",
    ).bind(request).bind(binding).fetch_optional(pool).await
}

pub async fn clear_retired(pool: &PgPool) -> sqlx::Result<()> {
    sqlx::query(
        "WITH ready AS (
           SELECT c.account_id, c.scope_id, c.adapter, c.binding_slot, c.version
           FROM gateway_provider_credentials c
           WHERE c.adapter IN ('connector_queue', 'openai_compatible_chat') AND c.retired_at IS NOT NULL AND c.encrypted_key IS NOT NULL
             AND NOT EXISTS (
               SELECT 1 FROM gateway_requests r
               LEFT JOIN gateway_provider_runs p ON p.request_id = r.id
               WHERE (r.account_id, r.provider_credential_scope_id, r.adapter, r.provider_credential_binding_slot,
                      r.provider_credential_version) = (c.account_id, c.scope_id, c.adapter, c.binding_slot, c.version)
                 AND (
                   num_nonnulls(r.succeeded_at, r.failed_at, r.uncertain_at, r.canceled_at) = 0
                   OR (r.adapter = 'connector_queue' AND r.content_removed_at IS NULL
                       AND r.uncertain_at IS NOT NULL
                       AND (
                           (p.upstream_handle IS NULL AND p.error_code = 'connector_submission_unconfirmed'
                             AND r.dispatched_at > statement_timestamp() - interval '16 minutes')
                           OR (p.upstream_handle IS NOT NULL AND (
                               (p.error_code = 'provider_poll_deadline' AND p.recheck_requested_at IS NULL
                                 AND r.uncertain_at > statement_timestamp() - interval '24 hours')
                               OR (p.recheck_requested_at IS NOT NULL AND p.poll_deadline IS NOT NULL)
                           ))
                       )
                   )
                 )
             )
           ORDER BY c.retired_at LIMIT 64 FOR UPDATE OF c SKIP LOCKED
         )
         UPDATE gateway_provider_credentials c SET encrypted_key = NULL
         FROM ready WHERE (c.account_id, c.scope_id, c.adapter, c.binding_slot, c.version) =
                          (ready.account_id, ready.scope_id, ready.adapter, ready.binding_slot, ready.version)",
    ).execute(pool).await?;
    Ok(())
}
