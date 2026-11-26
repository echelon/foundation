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
        workload: Workload::Mesh {
            format: "glb".into(),
        },
        provider: None,
        max_total_usd_nanos: None,
    }).await?;
    assert!(quote.selected_offer_id.is_some());
    let key = format!("rust-mesh-{}", SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos());
    let receipt = client.submit_mesh(&quote, "Rust mesh verification", SubmitOptions::new(&key)).await?;
    assert_eq!(client.wait(&receipt.id, WaitOptions::default()).await?.status, RequestStatus::Succeeded);
    let media = client.mesh_result(&receipt.id).await?.result.mesh;
    assert!(media.mesh_count > 0 && media.primitive_count > 0);
    assert_eq!(media.content_type, "model/gltf-binary");
    assert_eq!(media.artifact.sha256.len(), 64);
    println!("PASS Rust SDK quoted, submitted, polled, and read the scoped GLB manifest");
    Ok(())
}
