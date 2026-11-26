use ids::GatewayRequestId;
use sqlx::{Postgres, Transaction};

use super::RequestScope;

pub enum Recheck {
    Missing,
    Ineligible,
    Scheduled,
}

pub async fn recheck_provider(
    tx: &mut Transaction<'_, Postgres>,
    scope: RequestScope<'_>,
    request: &GatewayRequestId,
) -> sqlx::Result<Recheck> {
    let row = sqlx::query!(
        r#"WITH scoped AS (
             SELECT r.id, r.uncertain_at, r.adapter, r.content_removed_at,
                    r.provider_credential_version, r.provider_credential_scope_id,
                    r.provider_credential_binding_slot, r.account_id
             FROM gateway_requests r
             WHERE r.id = $1 AND r.account_id = $2 AND r.project_id = $3
               AND ($4::text IS NULL OR r.client_ref = $4)
             FOR UPDATE OF r
           ), scheduled AS (
             UPDATE gateway_provider_runs p SET
               recheck_requested_at = statement_timestamp(),
               recheck_unconfirmed_at = r.uncertain_at,
               poll_after = statement_timestamp(),
               poll_deadline = statement_timestamp() + interval '15 minutes'
             FROM scoped r WHERE p.request_id = r.id
               AND r.adapter IN ('fal_queue', 'kinovi_queue', 'connector_queue')
               AND r.content_removed_at IS NULL
               AND r.uncertain_at > statement_timestamp() - interval '24 hours'
               AND r.uncertain_at <= statement_timestamp()
               AND p.error_code = 'provider_poll_deadline'
               AND p.accepted_at IS NOT NULL AND p.upstream_handle IS NOT NULL
               AND p.poll_deadline IS NULL AND p.lease_until IS NULL
               AND p.recheck_requested_at IS NULL
               AND (r.provider_credential_version IS NULL OR EXISTS (
                 SELECT 1 FROM gateway_provider_credentials c
                 WHERE (c.account_id, c.scope_id, c.adapter, c.binding_slot, c.version) =
                       (r.account_id, r.provider_credential_scope_id, r.adapter,
                        r.provider_credential_binding_slot, r.provider_credential_version)
                   AND c.binding_digest = p.binding_digest
                   AND c.encrypted_key IS NOT NULL AND c.revoked_at IS NULL
               ))
             RETURNING p.request_id
           )
           SELECT EXISTS (SELECT 1 FROM scoped) AS "found!",
                  EXISTS (SELECT 1 FROM scheduled) AS "scheduled!""#,
        request.as_str(),
        scope.account.as_str(),
        scope.project.as_str(),
        scope.client_ref,
    )
    .fetch_one(&mut **tx)
    .await?;
    Ok(match (row.found, row.scheduled) {
        (false, _) => Recheck::Missing,
        (true, false) => Recheck::Ineligible,
        (true, true) => Recheck::Scheduled,
    })
}
