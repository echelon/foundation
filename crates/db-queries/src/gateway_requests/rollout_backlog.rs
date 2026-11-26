use sqlx::{PgConnection, Result};

#[derive(sqlx::FromRow)]
pub struct BindingDependency {
    pub phase: String,
    pub adapter: String,
    pub offer_id: String,
    pub provider: String,
    pub upstream_model: String,
    pub binding_digest: String,
    pub requests: i64,
}

pub async fn binding_dependencies(
    conn: &mut PgConnection,
    limit: i64,
) -> Result<Vec<BindingDependency>> {
    sqlx::query_as(
        r#"WITH outstanding AS (
             SELECT r.adapter, r.offer_id, r.provider, p.upstream_model, p.binding_digest,
                    CASE
                      WHEN r.uncertain_at IS NULL AND r.dispatched_at IS NULL
                           AND p.request_payload IS NOT NULL
                           AND LEAST(r.created_at, r.dispatch_after) > now() - interval '15 minutes'
                        THEN 'queued'
                      WHEN r.uncertain_at IS NULL AND r.dispatched_at IS NOT NULL
                        THEN 'in_flight'
                      WHEN p.recheck_requested_at IS NOT NULL AND p.poll_deadline > now()
                        THEN 'in_flight'
                      WHEN r.adapter IN ('fal_queue', 'kinovi_queue', 'connector_queue')
                           AND r.content_removed_at IS NULL
                           AND r.uncertain_at > now() - interval '24 hours'
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
                        THEN 'recheck_window'
                      WHEN r.adapter = 'connector_queue' AND r.content_removed_at IS NULL
                           AND r.uncertain_at IS NOT NULL AND p.upstream_handle IS NULL
                           AND r.dispatched_at > now() - interval '15 minutes'
                           AND p.error_code IN ('connector_submission_unconfirmed', 'submission_interrupted')
                           AND (p.connector_recovery_enabled OR EXISTS (
                             SELECT 1 FROM gateway_connector_completions completion
                             WHERE completion.request_id = r.id AND completion.next_lookup_at IS NOT NULL
                           ))
                        THEN 'submission_recovery'
                    END AS phase
               FROM gateway_requests r JOIN gateway_provider_runs p ON p.request_id = r.id
              WHERE num_nonnulls(r.succeeded_at, r.failed_at, r.canceled_at) = 0
                AND (r.uncertain_at IS NULL OR r.uncertain_at > now() - interval '25 hours')
           )
           SELECT phase, adapter, offer_id, provider, upstream_model, binding_digest, count(*) AS requests
             FROM outstanding WHERE phase IS NOT NULL
            GROUP BY phase, adapter, offer_id, provider, upstream_model, binding_digest
            ORDER BY phase, adapter, offer_id, provider, upstream_model, binding_digest
            LIMIT $1"#,
    )
    .bind(limit)
    .fetch_all(conn)
    .await
}
