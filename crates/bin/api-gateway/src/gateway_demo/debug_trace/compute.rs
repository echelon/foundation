use serde::Serialize;

use super::{Log, RequestStatus, Step, Tone, model_label};
use crate::gateway_demo::catalog::valid_identifier;
use crate::gateway_demo::compute_evidence::{ComputeAttempt, ComputeOutcome, ComputeSelection};

#[derive(Serialize, utoipa::ToSchema)]
pub(super) struct ComputeTargetAttempt {
    step_id: String,
    selection: ComputeSelection,
}

pub(super) fn add_steps(
    steps: &mut Vec<Step>,
    records: Vec<ComputeAttempt>,
    request_status: RequestStatus,
) -> Vec<ComputeTargetAttempt> {
    let Some(provider_index) = steps.iter().position(|step| step.id == "provider") else {
        return vec![];
    };
    let count = records.len();
    let mut new_steps = Vec::with_capacity(count);
    let mut attempts = Vec::with_capacity(count);
    let mut parent = steps[provider_index].id.clone();
    for (index, record) in records.into_iter().enumerate() {
        let selection = record.selection;
        if !safe_selection(&selection) {
            continue;
        }
        let step_id = format!("compute-attempt-{}", index + 1);
        let mut step = Step::point(
            &step_id,
            format!("Compute attempt {}", index + 1),
            "compute dispatch",
        )
        .at(record.started_at)
        .after(parent);
        step.log(Log {
            at: record.started_at,
            code: "compute.dispatch_recorded",
            message: "The worker recorded a target under its lease before sending the request.",
            severity: Tone::Active,
        });
        step.field("Home cell", &selection.home_cell);
        step.field("Compute stack", &selection.stack);
        step.field("Compute target", &selection.target);
        step.field("Cloud", &selection.cloud);
        step.field("Execution region", &selection.region);
        step.field("Zone", &selection.zone);
        step.field("Configured priority", selection.priority);

        if let Some(result) = record.result {
            let (label, tone, message) = detail(result.response.status);
            step = step.until(Some(result.observed_at)).status(label, tone);
            step.optional("HTTP status", result.response.http_status);
            step.log(Log {
                at: result.observed_at,
                code: result.response.status.code(),
                message,
                severity: tone,
            });
        } else if request_status == RequestStatus::Running && index + 1 == count {
            step = step.until(None).status("Dispatch recorded", Tone::Active);
        } else {
            step = step.status("Submission outcome unrecorded", Tone::Warning);
        }
        parent = step_id.clone();
        new_steps.push(step);
        attempts.push(ComputeTargetAttempt { step_id, selection });
    }
    steps.splice(provider_index + 1..provider_index + 1, new_steps);
    attempts
}

fn safe_selection(selection: &ComputeSelection) -> bool {
    [
        &selection.home_cell,
        &selection.stack,
        &selection.target,
        &selection.cloud,
        &selection.region,
        &selection.zone,
    ]
    .into_iter()
    .all(|value| valid_identifier(value) && model_label(Some(value)).is_some())
}

fn detail(outcome: ComputeOutcome) -> (&'static str, Tone, &'static str) {
    match outcome {
        ComputeOutcome::ConnectionFailed => (
            "Connection failed",
            Tone::Warning,
            "No connection was established. The worker could try another eligible target.",
        ),
        ComputeOutcome::CapacityRejected => (
            "Capacity rejected before acceptance",
            Tone::Warning,
            "This target returned a rate limit covered by its pre-acceptance retry contract.",
        ),
        ComputeOutcome::ResponseStarted => (
            "Response started",
            Tone::Ok,
            "The target returned successful response headers. Completion is tracked on the request.",
        ),
        ComputeOutcome::UpstreamError => (
            "Upstream error response",
            Tone::Warning,
            "The target returned an error. The worker did not attempt compute fallback for this response.",
        ),
        ComputeOutcome::TransportUnconfirmed => (
            "Acceptance unconfirmed",
            Tone::Warning,
            "Transport ended without confirmed acceptance. The worker did not attempt compute fallback.",
        ),
        ComputeOutcome::DeadlineBeforeSend => (
            "Deadline before send",
            Tone::Warning,
            "The dispatch budget expired after the target was recorded and before the worker sent the request.",
        ),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn compute_evidence_keeps_the_chosen_target_and_partial_outcome_distinct() {
        let selection = json!({
            "home_cell": "home", "stack": "chat", "target": "primary",
            "cloud": "aws", "region": "east", "zone": "a", "priority": 10,
        });
        let mut standby = selection.clone();
        standby["target"] = json!("standby");
        standby["cloud"] = json!("gcp");
        standby["region"] = json!("central");
        standby["zone"] = json!("b");
        let at = "2026-11-25T12:00:00Z";
        let records = serde_json::from_value(json!([
            {
                "selection": selection, "started_at": at,
                "result": { "status": "capacity_rejected", "http_status": 429, "observed_at": at },
            },
            { "selection": standby, "started_at": at },
        ]))
        .unwrap();

        let mut steps = vec![Step::point("provider", "Provider", "worker")];
        let attempts = add_steps(&mut steps, records, RequestStatus::AwaitingReconciliation);
        assert_eq!(attempts.len(), 2);
        assert_eq!(attempts[1].selection.target, "standby");
        assert_eq!(attempts[1].selection.home_cell, "home");
        assert_eq!(steps[1].status, "Capacity rejected before acceptance");
        assert_eq!(
            steps[2].parent_id.as_deref(),
            Some(attempts[0].step_id.as_str())
        );
        assert_eq!(steps[2].status, "Submission outcome unrecorded");
        assert!(matches!(steps[2].kind, super::super::Timing::Point));
    }
}
