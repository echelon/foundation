use crate::api::requests::responses::get_response::GeneratedImage;

/// Persist a live-test result image to the OS temp dir and return its `file://` URL.
///
/// OpenAI's image tool returns BYTES (base64), never a hosted URL — so the printable
/// "result URL" for a live test is a local file you can click open.
#[cfg(test)]
pub fn save_result_image(image: &GeneratedImage, label: &str) -> anyhow::Result<String> {
  let ext = image.format.as_str();
  let nanos = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_nanos())
    .unwrap_or(0);
  let path = std::env::temp_dir().join(format!("openai_client_{label}_{nanos}.{ext}"));
  std::fs::write(&path, &image.bytes)?;
  Ok(format!("file://{}", path.display()))
}
