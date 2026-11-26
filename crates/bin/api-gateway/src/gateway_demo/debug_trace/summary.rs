use chrono::SecondsFormat;
use db_queries::gateway_webhooks::Delivery;
use serde::Serialize;

use super::{RequestStatus, RequestView, Step, Tone};

#[derive(Serialize, utoipa::ToSchema)]
#[schema(as = RequestTraceSummary)]
pub(super) struct Summary {
    headline: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
    next_check: Check,
    signals: Vec<Signal>,
}

#[derive(Serialize, utoipa::ToSchema)]
#[schema(as = RequestTraceNextCheck)]
struct Check {
    label: &'static str,
    step_id: String,
}

#[derive(Serialize, utoipa::ToSchema)]
#[schema(as = RequestTraceSignal)]
struct Signal {
    label: &'static str,
    value: &'static str,
    tone: Tone,
    step_id: String,
}

pub(super) fn summarize(
    request: &RequestView,
    delivery: Option<&Delivery>,
    latest_attempt: Option<(i32, Option<i16>)>,
    steps: &[Step],
) -> Summary {
    use RequestStatus as Status;
    use Tone as T;
    let upstream = request.upstream.as_ref();
    let acknowledged = upstream.is_some_and(|row| row.accepted_at.is_some());
    let completion_seen = upstream.is_some_and(|row| row.result_observed_at.is_some());
    let removed = request.content_removed_at.is_some();
    let status = request.status;
    let present = |id: &str| steps.iter().any(|step| step.id == id);
    let provider = if present("provider") {
        "provider"
    } else if present("outcome") {
        "outcome"
    } else {
        "queue"
    };
    let execution = if present("outcome") {
        "outcome"
    } else {
        provider
    };
    let publication = "artifact-publication";
    let webhook = "webhook";
    let latest_delivery = latest_attempt.map(|(number, _)| format!("webhook-attempt-{number}"));
    let delivery_step = latest_delivery
        .as_deref()
        .filter(|id| present(id))
        .unwrap_or(webhook);

    let (execution_state, execution_tone) = match status {
        Status::Queued => ("Queued", T::Active),
        Status::Running if completion_seen => ("Provider completed", T::Ok),
        Status::Running if acknowledged => ("Provider acknowledged", T::Active),
        Status::Running => ("Running", T::Active),
        Status::Succeeded => ("Succeeded", T::Ok),
        Status::Canceled => ("Canceled", T::Neutral),
        Status::Failed => ("Failed", T::Error),
        Status::AwaitingReconciliation if completion_seen => ("Provider completed", T::Ok),
        Status::AwaitingReconciliation if acknowledged => ("Outcome unconfirmed", T::Warning),
        Status::AwaitingReconciliation => ("Acceptance unconfirmed", T::Warning),
    };
    let mut signals = Vec::with_capacity(4);
    let mut signal = |label, value, tone, step_id: &str| {
        signals.push(Signal {
            label,
            value,
            tone,
            step_id: step_id.to_owned(),
        })
    };
    signal("Execution", execution_state, execution_tone, execution);
    if removed {
        signal(
            "Output",
            "Stored content removed",
            T::Neutral,
            "content-removal",
        );
    } else if completion_seen {
        let (value, tone) = match status {
            Status::Succeeded => ("Published", T::Ok),
            Status::AwaitingReconciliation => ("Publication unconfirmed", T::Warning),
            Status::Failed | Status::Canceled => ("Publication ended", execution_tone),
            _ => ("Publication pending", T::Active),
        };
        signal("Output", value, tone, publication);
    }
    if request.supplier_settlement.is_some() {
        signal("Supplier cost", "Settlement recorded", T::Ok, provider);
    } else if upstream.is_some_and(|row| row.reported_usd_nanos.is_some()) {
        signal("Supplier cost", "Provider reported", T::Neutral, provider);
    } else if request.mock_reported_usd_nanos.is_some() {
        signal("Supplier cost", "Mock reported", T::Neutral, provider);
    } else {
        signal("Supplier cost", "Unreported", T::Neutral, provider);
    }
    let delivery_state = delivery.map(|value| (value.state.as_str(), value.attempt_count));
    if let Some((state, attempts)) = delivery_state {
        let (value, tone) = match state {
            "pending" if attempts > 0 => ("Retry pending", T::Warning),
            "pending" => ("Pending", T::Active),
            "sending" => ("Sending", T::Active),
            "delivered" => ("Delivered", T::Ok),
            "dead" => ("Delivery stopped", T::Error),
            _ => ("Status unrecognized", T::Neutral),
        };
        signal("Webhook", value, tone, delivery_step);
    } else if request.webhook_subscription_id.is_some() {
        signal("Webhook", "Delivery unrecorded", T::Neutral, execution);
    }

    let (headline, detail, label, step) = match status {
        Status::Queued if request.dispatch_paused == Some(true) => (
            "Request is queued. Dispatch is paused.",
            None,
            "Inspect queue wait",
            "queue",
        ),
        Status::Queued => (
            "Request is waiting for dispatch.",
            None,
            "Inspect queue wait",
            "queue",
        ),
        Status::Running if completion_seen => (
            "Provider completed. Artifact publication is pending.",
            None,
            "Inspect artifact publication",
            publication,
        ),
        Status::Running if acknowledged => (
            "The provider acknowledged the job.",
            None,
            "Inspect provider status",
            provider,
        ),
        Status::Running => (
            "Provider dispatch started.",
            None,
            "Inspect dispatch",
            provider,
        ),
        Status::AwaitingReconciliation if completion_seen => (
            "Provider completed. Artifact publication is unconfirmed.",
            None,
            "Inspect artifact publication",
            publication,
        ),
        Status::AwaitingReconciliation if acknowledged => (
            "The provider acknowledged the job. Its outcome is unconfirmed.",
            None,
            "Inspect the original job",
            provider,
        ),
        Status::AwaitingReconciliation => (
            "Provider acceptance is unconfirmed.",
            Some("Check for upstream acknowledgment before submitting again.".into()),
            "Inspect submission evidence",
            provider,
        ),
        Status::Failed if completion_seen => (
            "Provider completed. Artifact publication ended in failure.",
            None,
            "Inspect artifact publication",
            publication,
        ),
        Status::Failed => (
            "Request failed.",
            None,
            "Inspect request outcome",
            execution,
        ),
        Status::Canceled => (
            "Request was canceled.",
            None,
            "Inspect request outcome",
            execution,
        ),
        Status::Succeeded => match delivery {
            Some(delivery) if delivery.state == "pending" && delivery.attempt_count > 0 => {
                let mut detail = String::new();
                if let Some(status) = latest_attempt.and_then(|(_, status)| status) {
                    detail.push_str(&format!("Latest attempt returned HTTP {status}. "));
                }
                detail.push_str(&format!(
                    "Retry eligible after {}.",
                    delivery
                        .available_at
                        .to_rfc3339_opts(SecondsFormat::Secs, true)
                ));
                (
                    "Request succeeded. A webhook retry is pending.",
                    Some(detail),
                    "Inspect webhook attempt",
                    delivery_step,
                )
            }
            Some(delivery) if delivery.state == "dead" => (
                "Request succeeded. Webhook delivery stopped.",
                None,
                "Inspect webhook attempt",
                delivery_step,
            ),
            Some(delivery) if delivery.state == "delivered" && delivery.attempt_count > 1 => (
                "Request succeeded. The webhook was delivered after retry.",
                None,
                "Inspect webhook delivery",
                delivery_step,
            ),
            Some(delivery) if matches!(delivery.state.as_str(), "pending" | "sending") => (
                "Request succeeded. Webhook delivery is in progress.",
                None,
                "Inspect webhook delivery",
                delivery_step,
            ),
            _ if removed => (
                "Request succeeded. Stored content was removed.",
                None,
                "Inspect content removal",
                "content-removal",
            ),
            _ if completion_seen => (
                "Request succeeded. Artifact publication completed.",
                None,
                "Inspect artifact publication",
                publication,
            ),
            _ => (
                "Request succeeded.",
                None,
                "Inspect request outcome",
                execution,
            ),
        },
    };
    Summary {
        headline,
        detail,
        next_check: Check {
            label,
            step_id: step.to_owned(),
        },
        signals,
    }
}
