use std::path::PathBuf;

use crate::creds::google_aistudio_api_key::GoogleAiStudioApiKey;

/// Load the Google Gemini key for `#[ignore]` live tests from
/// `{repo}/external/credentials/google_aistudio_api_key.txt` (whitespace trimmed).
#[cfg(test)]
pub fn get_test_api_key() -> anyhow::Result<GoogleAiStudioApiKey> {
  let path = credential_path("google_aistudio_api_key.txt");
  let raw = std::fs::read_to_string(&path).map_err(|e| {
    anyhow::anyhow!("failed to read {} ({e}) — drop your key there for live tests", path.display())
  })?;
  let trimmed = raw.trim();
  if trimmed.is_empty() {
    anyhow::bail!("credential file {} is empty", path.display());
  }
  Ok(GoogleAiStudioApiKey::new(trimmed.to_string()))
}

#[cfg(test)]
fn credential_path(file: &str) -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("../../../external/credentials")
    .join(file)
}
