use chrono::{DateTime, Utc};
use ids::{GatewayProjectId, GatewayRequestId};
use sqlx::PgExecutor;

use super::{CostState, RequestRow, RequestScope, RequestStatus};

#[derive(Default)]
pub struct RequestFilter<'a> {
    pub after: Option<&'a GatewayRequestId>,
    pub app_ref: Option<&'a str>,
    pub client_unattributed: bool,
    pub app_unattributed: bool,
    pub trace_id: Option<&'a str>,
    pub model: Option<&'a str>,
    pub alternate_model: Option<&'a str>,
    pub provider: Option<&'a str>,
    pub status: Option<RequestStatus>,
    pub cost_state: Option<CostState>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

macro_rules! request_rows {
    ($filter:literal, $($value:expr),+ $(,)?) => {
        sqlx::query_as!(
            RequestRow,
        r#"SELECT r.id AS "id: GatewayRequestId", r.project_id AS "project_id: GatewayProjectId", r.client_ref, r.quote, r.created_at,
                  r.app_ref, r.provider, lifecycle.status AS "status!: RequestStatus",
                  lifecycle.cost_state AS "cost_state!: CostState", r.webhook_subscription_id,
                  coalesce(p.upstream_handle, a.provider_handle) AS "provider_handle?",
                  dispatch.submitted_at AS "submitted_at?",
                  r.succeeded_at AS completed_at, r.canceled_at, a.reported_usd_nanos,
                  gateway_request_can_cancel(r) AS "can_cancel!",
                  r.adapter, r.failed_at, r.uncertain_at, p.upstream_model AS "upstream_model?",
                  p.reported_model, p.reported_provider, p.finish_reason, p.error_code,
                  p.result_expires_at, r.content_removed_at, u.input_tokens, u.output_tokens, u.amount_usd_nanos AS upstream_usd_nanos,
                  charge.charge_unit AS "charge_unit?", charge.charge_quantity::text AS "charge_quantity?",
                  charge.reported_at AS "charge_observed_at?",
                  p.accepted_at AS provider_accepted_at, p.queue_state AS provider_queue_state,
                  p.result_observed_at AS provider_result_observed_at,
                  CASE WHEN r.adapter IN ('fal_queue', 'kinovi_queue', 'connector_queue') AND p.accepted_at IS NOT NULL
                         AND p.upstream_handle IS NOT NULL AND p.error_code = 'provider_poll_deadline'
                         AND p.recheck_requested_at IS NULL AND r.content_removed_at IS NULL
                         AND (r.provider_credential_version IS NULL OR EXISTS (
                           SELECT 1 FROM gateway_provider_credentials c
                           WHERE (c.account_id, c.scope_id, c.adapter, c.binding_slot, c.version) =
                                 (r.account_id, r.provider_credential_scope_id, r.adapter,
                                  r.provider_credential_binding_slot, r.provider_credential_version)
                             AND c.binding_digest = p.binding_digest
                             AND c.encrypted_key IS NOT NULL AND c.revoked_at IS NULL
                         ))
                       THEN r.uncertain_at + interval '24 hours' END AS "provider_recheck_available_until?",
                  p.recheck_requested_at AS provider_recheck_requested_at,
                  p.recheck_unconfirmed_at AS provider_recheck_unconfirmed_at,
                  CASE WHEN p.recheck_requested_at IS NOT NULL THEN p.poll_deadline
                       END AS "provider_recheck_deadline?",
                  callback.received_at AS "provider_callback_at?",
                  callback.recovered_from AS "provider_callback_recovered_from?",
                  callback.reconciled_at AS "provider_callback_reconciled_at?", r.provider_credential_version,
                  (r.provider_credential_scope_id = r.account_id) AS "organization_credential?",
                  r.provider_credential_binding_slot,
                  (lifecycle.status = 'queued' AND
                   coalesce(control.pause_all OR r.provider = ANY(control.paused_providers), false)) AS "dispatch_paused!",
                  settled.revision AS "settlement_revision?", settled.amount_usd_nanos AS "settlement_amount?",
                  settled.recorded_at AS "settlement_recorded_at?", p.stream_started_at, p.stream_detached_at,
                  r.trace_id, r.admission_span_id, r.caller_trace_id, r.caller_span_id,
                  r.client_budget_revision, r.client_budget_limit_usd_nanos
           FROM gateway_requests r
           LEFT JOIN gateway_dispatch_controls control ON control.account_id = r.account_id
           LEFT JOIN gateway_provider_attempts a
             ON a.account_id = r.account_id AND a.request_id = r.id AND a.attempt_number = 1
           LEFT JOIN gateway_provider_runs p ON p.request_id = r.id
           LEFT JOIN gateway_usage_observations charge ON charge.account_id = r.account_id
             AND charge.request_id = r.id AND charge.attempt_number = 1 AND charge.source = 'connector_charge'
           LEFT JOIN gateway_provider_callback_receipts callback ON callback.request_id = r.id
           LEFT JOIN LATERAL (
             SELECT revision, amount_usd_nanos, recorded_at FROM gateway_supplier_settlements
             WHERE request_id = r.id ORDER BY revision DESC LIMIT 1
           ) settled ON true
           LEFT JOIN LATERAL (
             SELECT input_tokens, output_tokens, amount_usd_nanos FROM gateway_usage_observations
             WHERE account_id = r.account_id AND request_id = r.id AND attempt_number = 1
               AND source IN ('openrouter_report', 'openrouter_stream_partial',
                              'local_chat_report', 'local_chat_stream_partial')
             ORDER BY source LIMIT 1
           ) u ON true
           CROSS JOIN LATERAL (
             SELECT CASE WHEN r.adapter = 'mock' THEN a.submitted_at ELSE r.dispatched_at END AS submitted_at
           ) dispatch
           CROSS JOIN LATERAL (
             SELECT CASE
                      WHEN r.uncertain_at IS NOT NULL THEN 'awaiting_reconciliation'
                      WHEN r.failed_at IS NOT NULL THEN 'failed'
                      WHEN r.canceled_at IS NOT NULL THEN 'canceled'
                      WHEN r.succeeded_at IS NOT NULL THEN 'succeeded'
                      WHEN dispatch.submitted_at IS NOT NULL THEN 'running'
                      ELSE 'queued'
                    END AS status,
                    CASE
                      WHEN settled.amount_usd_nanos IS NOT NULL THEN 'supplier_settled'
                      WHEN u.amount_usd_nanos IS NOT NULL THEN 'provider_reported'
                      WHEN a.reported_usd_nanos IS NOT NULL THEN 'mock_reported'
                      ELSE 'estimated'
                    END AS cost_state
           ) lifecycle
           "# + $filter,
            $($value),+
        )
    };
}

pub async fn get_request<'e, E: PgExecutor<'e>>(
    exec: E,
    scope: RequestScope<'_>,
    id: &GatewayRequestId,
) -> sqlx::Result<Option<RequestRow>> {
    // An equality on a unique key stays indexable when PostgreSQL reuses a generic plan.
    request_rows!(
        r#"WHERE r.id = $1 AND r.account_id = $2 AND r.project_id = $3
             AND ($4::text IS NULL OR r.client_ref = $4)
           LIMIT 1"#,
        id.as_str(), scope.account.as_str(), scope.project.as_str(), scope.client_ref,
    )
    .fetch_optional(exec)
    .await
}

pub async fn list_requests<'e, E: PgExecutor<'e>>(
    exec: E,
    scope: RequestScope<'_>,
    filter: RequestFilter<'_>,
    limit: i64,
) -> sqlx::Result<Vec<RequestRow>> {
    request_rows!(
        r#"WHERE r.account_id = $1 AND r.project_id = $2 AND ($3::text IS NULL OR r.client_ref = $3)
             AND ($4::text IS NULL OR r.sid < (SELECT sid FROM gateway_requests
                  WHERE account_id = $1 AND project_id = $2 AND ($3::text IS NULL OR client_ref = $3) AND id = $4))
             AND ($6::text IS NULL OR r.app_ref = $6)
             AND ($7::text IS NULL OR r.model IN ($7, $8::text))
             AND ($9::text IS NULL OR r.provider = $9)
             AND ($10::text IS NULL OR lifecycle.status = $10)
             AND ($11::text IS NULL OR lifecycle.cost_state = $11)
             AND ($12::timestamptz IS NULL OR r.created_at >= $12)
             AND ($13::timestamptz IS NULL OR r.created_at < $13)
             AND ($14::text IS NULL OR r.trace_id = $14 OR r.caller_trace_id = $14)
             AND (NOT $15::boolean OR r.client_ref IS NULL)
             AND (NOT $16::boolean OR r.app_ref IS NULL)
           ORDER BY r.sid DESC LIMIT $5"#,
        scope.account.as_str(), scope.project.as_str(), scope.client_ref,
        filter.after.map(GatewayRequestId::as_str), limit,
        filter.app_ref, filter.model, filter.alternate_model, filter.provider,
        filter.status as _, filter.cost_state as _, filter.from, filter.to, filter.trace_id,
        filter.client_unattributed, filter.app_unattributed,
    )
    .fetch_all(exec)
    .await
}
