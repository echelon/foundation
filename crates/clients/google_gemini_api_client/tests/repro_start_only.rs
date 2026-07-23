//! Diagnostic: start ONE background Omni interaction via the real client path,
//! print the handle, do not poll.

use google_gemini_api_client::api::requests::videos::generate_gemini_omni_flash::generate_gemini_omni_flash::{
  generate_gemini_omni_flash, GeminiOmniFlashRequest, GenerateGeminiOmniFlashArgs,
};
use google_gemini_api_client::creds::google_gemini_api_key::GoogleGeminiApiKey;

#[tokio::test]
#[ignore]
async fn start_only() {
  let key = std::fs::read_to_string(
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../external/credentials/google_gemini_api_key.txt"),
  )
  .unwrap()
  .trim()
  .to_string();
  let request = GeminiOmniFlashRequest {
    prompt: "A very short clip: a snail crosses a leaf".to_string(),
    images: vec![],
    video_file_uri: None,
    task: None,
    aspect_ratio: None,
    delivery: None,
    previous_interaction_id: None,
    store: None,
    expected_output_seconds: None,
  };
  let handle =
    generate_gemini_omni_flash(GenerateGeminiOmniFlashArgs { api_key: &GoogleGeminiApiKey::new(key), request })
      .await
      .unwrap();
  println!("HANDLE>>>{}<<<", handle.0);
}
