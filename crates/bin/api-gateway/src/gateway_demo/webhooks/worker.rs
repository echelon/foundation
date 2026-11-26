use std::sync::Arc;
use std::time::{Duration, Instant};

use db_queries::gateway_webhooks::{self, Claim, DeliveryResult};
use tracing::Instrument;

use crate::middleware::trace_context::work_span;

use super::config::WebhookConfig;

pub fn spawn(pool: db::PgPool, config: Arc<WebhookConfig>) {
    tokio::spawn(async move {
        let mut tasks = tokio::task::JoinSet::new();
        let mut timer = tokio::time::interval(Duration::from_millis(250));
        timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            timer.tick().await;
            while let Some(result) = tasks.try_join_next() {
                if result.is_err() {
                    tracing::warn!("webhook task stopped before writing its outcome");
                }
            }
            if tasks.len() >= 4 {
                continue;
            }
            let claim = match gateway_webhooks::claim(&pool).await {
                Ok(Some(claim)) => claim,
                Ok(None) => continue,
                Err(error) => {
                    log_database_error(&error);
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    continue;
                }
            };
            let pool = pool.clone();
            let config = Arc::clone(&config);
            let span = work_span(
                "webhook_delivery", &claim.request_id, claim.trace_id.as_deref(), claim.admission_span_id.as_deref(),
            );
            tasks.spawn(async move {
                let started = Instant::now();
                let result = send(&config, &claim).await;
                if let Err(error) = gateway_webhooks::finish(&pool, &claim, result).await {
                    log_database_error(&error);
                } else {
                    tracing::debug!(
                        elapsed_ms = started.elapsed().as_millis() as u64,
                        attempt = claim.attempt,
                        "webhook delivery cycle finished"
                    );
                }
            }.instrument(span));
        }
    });
}

async fn send(config: &WebhookConfig, claim: &Claim) -> DeliveryResult {
    let Some(url) = config.matches_destination(&claim.destination_id, &claim.destination_url)
    else {
        return DeliveryResult::DestinationUnavailable;
    };
    let timestamp = chrono::Utc::now().timestamp().to_string();
    let response = config
        .client
        .post(url.clone())
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header("webhook-id", &claim.id)
        .header("webhook-timestamp", &timestamp)
        .header("webhook-signature", config.signature(claim, &timestamp))
        .body(claim.payload.clone())
        .send()
        .await;
    let backoff = 1_000 * (1 << (claim.tries_in_cycle - 1)) + rand::random::<i32>().rem_euclid(250);
    let Ok(response) = response else {
        return DeliveryResult::NetworkError {
            retry_after_ms: backoff,
        };
    };
    let status = response.status();
    let code = status.as_u16() as i16;
    if status.is_success() {
        return DeliveryResult::Delivered(code);
    }
    let retry_after_ms = (status.is_server_error() || matches!(code, 408 | 425 | 429)).then(|| {
        let requested = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|header| header.to_str().ok())
            .and_then(|seconds| seconds.parse::<u64>().ok())
            .map_or(0, |seconds| seconds.min(60) as i32 * 1_000);
        backoff.max(requested)
    });
    DeliveryResult::HttpError {
        status: code,
        retry_after_ms,
    }
}

fn log_database_error(error: &sqlx::Error) {
    tracing::warn!(sqlstate = ?error.as_database_error().and_then(|error| error.code()), "webhook worker database operation failed");
}
