use reqwest::header::HeaderValue;
use serde::{Deserialize, Serialize};

use crate::{
    DownstreamProvider, Error, Provider, ProviderAttempt, ProviderAttemptState, RequestStatus,
    TraceCorrelation,
};

pub(super) fn submit_traceparent(value: &str) -> Result<HeaderValue, Error> {
    let invalid = || Error::InvalidInput("supply a valid W3C traceparent");
    if !valid_traceparent(value) {
        return Err(invalid());
    }
    let mut header = HeaderValue::from_str(value).map_err(|_| invalid())?;
    header.set_sensitive(true);
    Ok(header)
}

fn valid_traceparent(value: &str) -> bool {
    if value.len() > 256 || !value.bytes().all(|byte| byte.is_ascii_graphic() && byte != b',') {
        return false;
    }
    let Some(base) = value.get(..55) else {
        return false;
    };
    let mut fields = base.split('-');
    let (Some(version), Some(trace), Some(parent), Some(flags), None) =
        (fields.next(), fields.next(), fields.next(), fields.next(), fields.next())
    else {
        return false;
    };
    let hex = |field: &str, length| {
        field.len() == length && field.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    };
    let suffix = &value[55..];
    hex(version, 2) && version != "ff"
        && hex(trace, 32) && trace != "00000000000000000000000000000000"
        && hex(parent, 16) && parent != "0000000000000000"
        && hex(flags, 2)
        && (suffix.is_empty() || (version != "00" && suffix.starts_with('-')))
}

#[derive(Clone, Debug, Deserialize)]
pub struct RequestTraceSnapshot {
    pub revision: String,
    pub observed_at: String,
    pub analysis_available: bool,
    pub evidence: TraceEvidence,
    pub provider_path: Option<TraceProviderPath>,
    pub correlation: Option<TraceCorrelation>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TraceProviderPath {
    pub admitted_offer_id: Option<String>,
    pub attempts: Vec<TraceProviderAttempt>,
    pub downstream: Option<DownstreamProvider>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TraceProviderAttempt {
    #[serde(flatten)]
    pub attempt: ProviderAttempt,
    #[serde(default)]
    pub compute_attempts: Vec<ComputeTargetAttempt>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ComputeTargetAttempt {
    pub step_id: String,
    pub selection: ComputeSelection,
}

/// Placement declared in the gateway's configured target.
#[derive(Clone, Debug, Deserialize)]
pub struct ComputeSelection {
    pub home_cell: String,
    pub stack: String,
    pub target: String,
    pub cloud: String,
    pub region: String,
    pub zone: String,
    pub priority: u8,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TraceEvidence {
    pub schema_version: u8,
    pub request_id: String,
    pub status: RequestStatus,
    pub subscription_configured: bool,
    pub delivery_status: Option<String>,
    pub delivery_attempts_omitted: usize,
    pub steps: Vec<TraceStep>,
    pub coverage: Vec<String>,
    pub summary: Option<TraceSummary>,
    pub routing: Option<TraceRouting>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TraceStep {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub component: String,
    pub kind: TraceTiming,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub status: String,
    pub tone: TraceTone,
    pub logs: Vec<TraceLog>,
    pub attributes: Vec<TraceAttribute>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TraceTiming {
    Point,
    Interval,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TraceTone {
    Ok,
    Active,
    Warning,
    Error,
    Neutral,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TraceLog {
    pub at: String,
    pub code: String,
    pub message: String,
    pub severity: TraceTone,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TraceAttribute {
    pub label: String,
    pub value: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TraceSummary {
    pub headline: String,
    pub detail: Option<String>,
    pub next_check: TraceCheck,
    pub signals: Vec<TraceSignal>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TraceCheck {
    pub label: String,
    pub step_id: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TraceSignal {
    pub label: String,
    pub value: String,
    pub tone: TraceTone,
    pub step_id: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TraceRouting {
    pub admitted_offer_id: Option<String>,
    pub attempts: Vec<TraceRouteAttempt>,
    pub downstream: Option<DownstreamProvider>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TraceRouteAttempt {
    pub offer_id: String,
    pub provider: Option<Provider>,
    pub state: ProviderAttemptState,
    pub step_id: String,
    pub requested_model: Option<String>,
    pub reported_model: Option<String>,
    pub acknowledged_at: Option<String>,
}

/// OTLP/HTTP JSON ExportTraceServiceRequest.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OtlpTrace {
    pub resource_spans: Vec<serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TraceBaseline {
    pub model: String,
    pub provider: Provider,
    pub offer_id: String,
    pub window_start: String,
    pub window_end: String,
    pub request_ids: Vec<String>,
    pub timings: Vec<TraceBaselineTiming>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TraceBaselineTiming {
    pub metric: TraceBaselineMetric,
    pub request_ms: Option<u64>,
    pub median_ms: Option<u64>,
    pub samples: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TraceBaselineMetric {
    Elapsed,
    Queue,
    Publication,
}
