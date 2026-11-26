use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Operator-configured placement recorded at dispatch. Contains no destination or credential.
#[derive(Deserialize, Serialize, utoipa::ToSchema)]
pub(super) struct ComputeSelection {
    pub home_cell: String,
    pub stack: String,
    pub target: String,
    pub cloud: String,
    pub region: String,
    pub zone: String,
    /// Lower numbers have precedence. Ties are distributed by stable request hash.
    pub priority: u8,
}

#[derive(Deserialize)]
pub(super) struct ComputeAttempt {
    pub selection: ComputeSelection,
    pub started_at: DateTime<Utc>,
    pub result: Option<ComputeResult>,
}

#[derive(Deserialize)]
pub(super) struct ComputeResult {
    #[serde(flatten)]
    pub response: ComputeResponse,
    pub observed_at: DateTime<Utc>,
}

#[derive(Deserialize, Serialize)]
pub(super) struct ComputeResponse {
    pub status: ComputeOutcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_status: Option<u16>,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ComputeOutcome {
    ConnectionFailed,
    CapacityRejected,
    ResponseStarted,
    UpstreamError,
    TransportUnconfirmed,
    DeadlineBeforeSend,
}

impl ComputeOutcome {
    pub(super) fn code(self) -> &'static str {
        match self {
            Self::ConnectionFailed => "compute_connection_failed",
            Self::CapacityRejected => "compute_capacity_rejected",
            Self::ResponseStarted => "response_started",
            Self::UpstreamError => "upstream_error",
            Self::TransportUnconfirmed => "transport_unconfirmed",
            Self::DeadlineBeforeSend => "compute_dispatch_deadline",
        }
    }
}
