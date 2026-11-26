use std::time::Duration;

use db_queries::gateway_requests::compute_attempts::{
    ComputeAttemptFinish, ComputeAttemptLease, append_compute_attempt, finish_compute_attempt,
};
use ids::GatewayRequestId;

use crate::gateway_demo::compute_evidence::ComputeResponse;

use super::super::local_chat::compute::{ComputeStack, Target};
use super::{ChatDispatch, ComputeOutcome, SubmitOutcome};

const WRITE_TIMEOUT: Duration = Duration::from_secs(1);

pub(super) struct ComputeTrace<'a> {
    dispatch: &'a ChatDispatch<'a>,
    request: Option<GatewayRequestId>,
}

pub(super) struct Attempt<'a> {
    target: &'a Target,
    stack: &'a ComputeStack,
    stored: Option<StoredAttempt<'a>>,
}

struct StoredAttempt<'a> {
    pool: &'a db::PgPool,
    lease: ComputeAttemptLease<'a>,
    slot: i32,
}

impl<'a> ComputeTrace<'a> {
    pub(super) fn new(dispatch: &'a ChatDispatch<'a>) -> Result<Self, SubmitOutcome> {
        let request = if dispatch.pool.is_some() {
            Some(dispatch.request_id.parse().map_err(|_| unavailable())?)
        } else {
            None
        };
        Ok(Self { dispatch, request })
    }

    pub(super) async fn start<'b>(
        &'b self,
        target: &'b Target,
        stack: &'b ComputeStack,
        budget: Duration,
    ) -> Result<Attempt<'b>, SubmitOutcome> {
        let stored = if let (Some(pool), Some(request)) = (self.dispatch.pool, &self.request) {
            let lease = ComputeAttemptLease {
                request,
                until: self.dispatch.lease,
            };
            let selection =
                serde_json::to_value(target.selection(stack)).map_err(|_| unavailable())?;
            let slot = match tokio::time::timeout(
                budget.min(WRITE_TIMEOUT),
                append_compute_attempt(pool, lease, &selection),
            )
            .await
            {
                Ok(Ok(Some(slot))) => slot,
                _ => {
                    tracing::warn!("could not record compute dispatch");
                    return Err(unavailable());
                }
            };
            Some(StoredAttempt { pool, lease, slot })
        } else {
            None
        };
        Ok(Attempt {
            target,
            stack,
            stored,
        })
    }
}

impl Attempt<'_> {
    pub(super) async fn record(self, outcome: ComputeOutcome, http_status: Option<u16>) {
        self.target.record(self.stack, outcome);
        if let Some(stored) = self.stored {
            let Ok(result) = serde_json::to_value(ComputeResponse {
                status: outcome,
                http_status,
            }) else {
                return;
            };
            let write = finish_compute_attempt(
                stored.pool,
                ComputeAttemptFinish {
                    lease: stored.lease,
                    slot: stored.slot,
                    result: &result,
                },
            );
            if !matches!(
                tokio::time::timeout(WRITE_TIMEOUT, write).await,
                Ok(Ok(true))
            ) {
                // The response still governs acceptance and retry safety when evidence is incomplete.
                tracing::warn!("could not record compute submission outcome");
            }
        }
    }
}

fn unavailable() -> SubmitOutcome {
    SubmitOutcome::Rejected("compute_dispatch_evidence_unavailable")
}
