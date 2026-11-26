use axum::extract::{Query, State};
use axum::Json;
use chrono::{DateTime, Days, NaiveDate, Utc};
use db_queries::gateway_requests::{report_usage, UsageFilter, UsageRow};
use ids::GatewayProjectId;
use serde::{Deserialize, Serialize};

use super::access::{self, GatewayCaller, Permission, ScopeQuery};
use super::catalog::{alternate_model_id, valid_identifier, Provider};
use super::requests::validate_app_ref;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

const PAGE_SIZE: usize = 50;

#[derive(Clone, Copy, Default, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
enum GroupBy {
    #[default]
    Provider,
    Model,
    Client,
    App,
    Day,
}

impl GroupBy {
    fn as_str(self) -> &'static str {
        match self {
            Self::Provider => "provider",
            Self::Model => "model",
            Self::Client => "client",
            Self::App => "app",
            Self::Day => "day",
        }
    }
}

#[derive(Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct UsageQuery {
    account_id: Option<String>,
    project_id: Option<String>,
    /// Inclusive UTC date. Defaults to seven days before to.
    from: Option<NaiveDate>,
    /// Exclusive UTC date. Defaults to tomorrow in UTC.
    to: Option<NaiveDate>,
    client_ref: Option<String>,
    /// Exact application reference.
    app_ref: Option<String>,
    provider: Option<Provider>,
    model: Option<String>,
    #[serde(default)]
    /// Group results by provider, model, client, app, or UTC day. Defaults to provider.
    group_by: GroupBy,
    /// Use the previous page’s next_after value with the same filters.
    after: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
struct ReportWindow {
    from: NaiveDate,
    to: NaiveDate,
}

impl ReportWindow {
    fn resolve(from: Option<NaiveDate>, to: Option<NaiveDate>) -> ApiResult<Self> {
        let invalid =
            || ApiError::BadRequest("use a positive UTC date range of up to 92 days".into());
        let to = to
            .or_else(|| Utc::now().date_naive().succ_opt())
            .ok_or_else(invalid)?;
        let from = from
            .or_else(|| to.checked_sub_days(Days::new(7)))
            .ok_or_else(invalid)?;
        if !(1..=92).contains(&(to - from).num_days()) {
            return Err(invalid());
        }
        Ok(Self { from, to })
    }
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct UsageMetrics {
    requests: i64,
    succeeded: i64,
    canceled: i64,
    failed: i64,
    awaiting_reconciliation: i64,
    in_progress: i64,
    provider_requests: i64,
    completed_outputs: i64,
    estimated_usd_nanos: String,
    in_progress_estimated_usd_nanos: String,
    mock_reported_usd_nanos: String,
    provider_reported_usd_nanos: Option<String>,
    provider_reports_missing_cost: i64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    provider_charges: Vec<UsageChargeTotal>,
    supplier_settled_usd_nanos: Option<String>,
    supplier_settled_requests: i64,
    provider_input_tokens: Option<i64>,
    provider_output_tokens: Option<i64>,
    avg_success_latency_ms: Option<i64>,
}

#[derive(Serialize, utoipa::ToSchema)]
struct UsageChargeTotal {
    provider: String,
    unit: String,
    quantity: String,
    observations: i64,
}

impl From<UsageRow> for UsageMetrics {
    fn from(row: UsageRow) -> Self {
        Self {
            requests: row.requests,
            succeeded: row.succeeded,
            canceled: row.canceled,
            failed: row.failed,
            awaiting_reconciliation: row.awaiting_reconciliation,
            in_progress: row.in_progress,
            provider_requests: row.provider_requests,
            completed_outputs: row.completed_outputs,
            estimated_usd_nanos: row.estimated_usd_nanos,
            in_progress_estimated_usd_nanos: row.in_progress_estimated_usd_nanos,
            mock_reported_usd_nanos: row.mock_reported_usd_nanos,
            provider_reported_usd_nanos: row.provider_reported_usd_nanos,
            provider_reports_missing_cost: row.provider_reports_missing_cost,
            provider_charges: row.provider_charges.0.into_iter().map(|charge| UsageChargeTotal {
                provider: charge.provider, unit: charge.unit, quantity: charge.quantity, observations: charge.observations,
            }).collect(),
            supplier_settled_usd_nanos: row.supplier_settled_usd_nanos,
            supplier_settled_requests: row.supplier_settled_requests,
            provider_input_tokens: row.provider_input_tokens,
            provider_output_tokens: row.provider_output_tokens,
            avg_success_latency_ms: row.avg_success_latency_ms,
        }
    }
}

#[derive(Serialize, utoipa::ToSchema)]
struct UsageGroup {
    key: Option<String>,
    #[serde(flatten)]
    metrics: UsageMetrics,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct UsageReport {
    #[schema(value_type = String)]
    project_id: GatewayProjectId,
    window: ReportWindow,
    as_of: DateTime<Utc>,
    source: &'static str,
    currency: &'static str,
    group_by: GroupBy,
    client_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    app_ref: Option<String>,
    provider: Option<Provider>,
    model: Option<String>,
    totals: UsageMetrics,
    groups: Vec<UsageGroup>,
    next_after: Option<String>,
}

#[utoipa::path(
    get, path = "/v1/usage", operation_id = "getGatewayUsage", tag = "native",
    summary = "Report project usage",
    description = "A project key requires usage:read. Totals cover the full filtered window. The report separates estimates, reported costs, and supplier settlements.",
    security(("project_api_key" = []), ("session_cookie" = []), ("api_key" = [])),
    params(UsageQuery),
    responses(
        (status = 200, description = "Usage totals, grouped rows, and a cursor for more groups.", body = UsageReport),
        (status = "default", response = super::RequestErrorResponse),
    ),
)]
pub async fn report(
    State(state): State<AppState>,
    caller: GatewayCaller,
    Query(mut query): Query<UsageQuery>,
) -> ApiResult<Json<UsageReport>> {
    let requested = ScopeQuery {
        account_id: query.account_id.take(),
        project_id: query.project_id.take(),
    };
    let access = access::resolve(&state, caller, requested, Permission::ReadUsage).await?;
    Ok(Json(report_for(&state.pool, &access, query).await?))
}

pub(super) async fn report_for(
    pool: &db::PgPool,
    access: &access::Access,
    query: UsageQuery,
) -> ApiResult<UsageReport> {
    let scope = access.for_client(query.client_ref.as_deref())?;
    let app_ref = query.app_ref.as_deref();
    validate_app_ref(app_ref)?;
    let model = query.model.as_deref();
    if model.is_some_and(|model| !valid_identifier(model)) {
        return Err(ApiError::BadRequest("invalid model filter".into()));
    }
    if query
        .after
        .as_deref()
        .is_some_and(|after| !valid_identifier(after))
    {
        return Err(ApiError::BadRequest("invalid group cursor".into()));
    }

    let window = ReportWindow::resolve(query.from, query.to)?;
    let as_of = Utc::now();
    let mut rows = report_usage(
        pool,
        scope,
        UsageFilter {
            from: window.from.and_time(chrono::NaiveTime::MIN).and_utc(),
            to: window.to.and_time(chrono::NaiveTime::MIN).and_utc(),
            provider: query.provider.as_ref().map(Provider::as_str),
            model,
            alternate_model: model.and_then(alternate_model_id),
            app_ref,
            group_by: query.group_by.as_str(),
            after: query.after.as_deref(),
        },
        (PAGE_SIZE + 2) as i64,
    )
    .await?
    .into_iter();
    let totals = rows
        .next()
        .filter(|row| row.is_total)
        .ok_or_else(|| ApiError::Internal("usage report missing totals".into()))?;
    let groups: Vec<_> = rows
        .by_ref()
        .take(PAGE_SIZE)
        .map(|mut row| UsageGroup {
            key: row.group_key.take(),
            metrics: row.into(),
        })
        .collect();
    let next_after = if rows.next().is_some() {
        groups.last().and_then(|group| group.key.clone())
    } else {
        None
    };
    let source = if totals.provider_requests == 0 {
        "mock_provider"
    } else if totals.provider_requests == totals.requests {
        "provider"
    } else {
        "mixed"
    };
    Ok(UsageReport {
        project_id: scope.project.clone(),
        window,
        as_of,
        source,
        currency: "USD",
        group_by: query.group_by,
        client_ref: scope.client_ref.map(str::to_owned),
        app_ref: query.app_ref,
        provider: query.provider,
        model: query.model,
        totals: totals.into(),
        groups,
        next_after,
    })
}
