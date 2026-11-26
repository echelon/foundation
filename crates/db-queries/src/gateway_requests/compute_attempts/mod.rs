use chrono::{DateTime, Utc};
use ids::GatewayRequestId;

mod append_compute_attempt;
mod finish_compute_attempt;
mod read_compute_attempts;

pub use append_compute_attempt::append_compute_attempt;
pub use finish_compute_attempt::{ComputeAttemptFinish, finish_compute_attempt};
pub use read_compute_attempts::read_compute_attempts;

#[derive(Clone, Copy)]
pub struct ComputeAttemptLease<'a> {
    pub request: &'a GatewayRequestId,
    pub until: DateTime<Utc>,
}
