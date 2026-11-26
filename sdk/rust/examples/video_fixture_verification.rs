use std::time::{SystemTime, UNIX_EPOCH};
use tokencraft_sdk::{QuoteInput, RequestStatus, SubmitOptions, TokenCraftClient, WaitOptions, Workload};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("TOKENCRAFT_API_KEY")?;
    let model = std::env::var("TOKENCRAFT_MODEL")?;
    let base_url =
        std::env::var("TOKENCRAFT_BASE_URL").unwrap_or_else(|_| "http://localhost:4000".into());
    let client = TokenCraftClient::new(&base_url, Some(&token))?;
    let quote = client.quote(&QuoteInput {
        model,
        workload: Workload::Video {
            duration_seconds: 5,
            aspect_ratio: "16:9".into(),
            resolution: "540p".into(),
            max_relative_error_bps: None,
        },
        provider: None,
        max_total_usd_nanos: None,
    }).await?;
    assert!(quote.selected_offer_id.is_some());
    let key = format!("rust-video-{}", SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos());
    let receipt = client.submit_video(&quote, "Rust video verification", SubmitOptions::new(&key)).await?;
    assert_eq!(client.wait(&receipt.id, WaitOptions::default()).await?.status, RequestStatus::Succeeded);
    let media = client.video_result(&receipt.id).await?.result.video;
    assert_eq!((media.width, media.height, media.content_type.as_str()), (960, 540, "video/mp4"));
    assert_eq!(media.artifact.sha256.len(), 64);
    println!("PASS Rust SDK quoted, submitted, polled, and read the scoped MP4 manifest");
    Ok(())
}
