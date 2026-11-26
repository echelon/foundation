use axum::{
    extract::{Path, Query, State},
    http::header,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use db_queries::gateway_requests;
use serde::Serialize;

use super::{
    access::{self, GatewayCaller, Permission, ScopeQuery},
    parse_request_id,
};
use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

#[derive(Serialize, utoipa::ToSchema)]
#[schema(as = RequestTraceBaseline)]
struct Baseline {
    model: String,
    provider: String,
    offer_id: String,
    window_start: DateTime<Utc>,
    window_end: DateTime<Utc>,
    request_ids: Vec<String>,
    timings: [TimingComparison; 3],
}

#[derive(Serialize, utoipa::ToSchema)]
#[schema(as = RequestTraceTimingComparison)]
struct TimingComparison {
    metric: TimingMetric,
    request_ms: Option<i64>,
    median_ms: Option<i64>,
    samples: i64,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = RequestTraceTimingMetric)]
enum TimingMetric {
    Elapsed,
    Queue,
    Publication,
}

#[utoipa::path(
    get, path = "/v1/requests/{id}/trace/baseline", operation_id = "getGatewayRequestTraceBaseline", tag = "native",
    summary = "Compare recorded timings with earlier successful requests in the same project and client",
    security(("project_api_key" = []), ("session_cookie" = []), ("api_key" = [])),
    params(("id" = String, Path, description = "Durable request ID."), ScopeQuery),
    responses(
        (status = 200, description = "Timing comparison and scoped sample IDs.", body = Baseline),
        (status = 404, description = "Request is outside scope or unavailable.", body = crate::error::GatewayError),
        (status = "default", response = super::super::RequestErrorResponse),
    ),
)]
pub(in crate::gateway_demo) async fn get(
    State(state): State<AppState>,
    caller: GatewayCaller,
    Query(scope): Query<ScopeQuery>,
    Path(raw): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let access = access::resolve(&state, caller, scope, Permission::ReadRequests).await?;
    let id = parse_request_id(&raw)?;
    let row = gateway_requests::trace_baseline(&state.pool, access.scope(), &id)
        .await?
        .ok_or_else(|| ApiError::NotFound("request not found".into()))?;
    let timing = |metric, request_ms, median_ms, samples| TimingComparison {
        metric,
        request_ms,
        median_ms,
        samples,
    };
    let baseline = Baseline {
        model: row.model,
        provider: row.provider,
        offer_id: row.offer_id,
        window_start: row.window_start,
        window_end: row.window_end,
        request_ids: row.request_ids,
        timings: [
            timing(
                TimingMetric::Elapsed,
                row.total_ms,
                row.total_median_ms,
                row.total_samples,
            ),
            timing(
                TimingMetric::Queue,
                row.queue_ms,
                row.queue_median_ms,
                row.queue_samples,
            ),
            timing(
                TimingMetric::Publication,
                row.publication_ms,
                row.publication_median_ms,
                row.publication_samples,
            ),
        ],
    };
    Ok(([(header::CACHE_CONTROL, "no-store")], Json(baseline)))
}
