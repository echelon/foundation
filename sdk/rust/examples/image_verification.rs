use tokencraft_sdk::{QuoteInput, SubmitOptions, TokenCraftClient, WaitOptions, Workload};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = std::env::var("TOKENCRAFT_API_KEY")?;
    let idempotency_key = std::env::var("TOKENCRAFT_VERIFICATION_ID")?;
    let base_url =
        std::env::var("TOKENCRAFT_BASE_URL").unwrap_or_else(|_| "http://localhost:4000".into());
    let model = std::env::var("TOKENCRAFT_MODEL").unwrap_or_else(|_| "example_image".into());
    let client = TokenCraftClient::new(&base_url, Some(&key))?;
    let quote = client
        .quote(&QuoteInput {
            model,
            workload: Workload::Image {
                count: 2,
                aspect_ratio: "4:3".into(),
                max_relative_error_bps: None,
            },
            provider: None,
            max_total_usd_nanos: None,
        })
        .await?;
    assert!(
        quote.selected_offer_id.is_some(),
        "expected an eligible image offer"
    );
    let mut options = SubmitOptions::new(&idempotency_key);
    options.image_prompt = Some("Synthetic scene for Rust SDK verification");
    let receipt = client.submit(&quote, options).await?;
    client.wait(&receipt.id, WaitOptions::default()).await?;
    let result = client.image_result(&receipt.id).await?;
    assert_eq!(result.result.images.len(), 2);
    assert_eq!(result.result.images[0].content_type, "image/png");
    println!("PASS Rust SDK: quote, prompt submission, and two typed outputs");
    Ok(())
}
