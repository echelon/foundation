use std::{env, error::Error, fs, time::Duration};

use tokencraft_sdk::{Quote, QuoteInput, SubmitOptions, TokenCraftClient, WaitOptions, Workload};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let base = env::var("TOKENCRAFT_BASE_URL").unwrap_or_else(|_| "http://localhost:4000".into());
    let key = env::var("TOKENCRAFT_API_KEY")?;
    let client = TokenCraftClient::new(&base, Some(&key))?;
    let args = env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [command] if command == "quote" => {
            let quote = client
                .quote(&QuoteInput {
                    model: "studio_image_demo".into(),
                    workload: Workload::Image {
                        count: 1,
                        aspect_ratio: "7:5".into(),
                        max_relative_error_bps: None,
                    },
                    provider: None,
                    max_total_usd_nanos: None,
                })
                .await?;
            println!("{}", serde_json::to_string(&quote)?);
        }
        [command, path, job_key] if command == "submit" => {
            let quote: Quote = serde_json::from_slice(&fs::read(path)?)?;
            let receipt = client.submit(&quote, SubmitOptions::new(job_key)).await?;
            println!("{}", serde_json::to_string(&receipt)?);
        }
        [command, request_id] if command == "wait" => {
            let trace = client
                .wait(
                    request_id,
                    WaitOptions {
                        timeout: Duration::from_secs(15),
                        poll_interval: Duration::from_millis(250),
                    },
                )
                .await?;
            println!(
                "{}",
                serde_json::json!({
                    "id": trace.id,
                    "status": trace.status,
                    "client_ref": trace.client_ref,
                    "offer": trace.quote.selected_offer_id,
                    "reported_usd_nanos": trace.mock_reported_usd_nanos,
                    "event_count": trace.events.len(),
                })
            );
        }
        _ => {
            return Err(
                "use quote, submit <quote.json> <idempotency-key>, or wait <request-id>".into(),
            )
        }
    }
    Ok(())
}
