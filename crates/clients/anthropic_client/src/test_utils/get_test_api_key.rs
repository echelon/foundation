use std::path::PathBuf;

use crate::creds::anthropic_api_key::AnthropicApiKey;

/// Load the Anthropic key for `#[ignore]` live tests from
/// `{repo}/external/credentials/anthropic_api_key.txt` (whitespace trimmed).
#[cfg(test)]
pub fn get_test_api_key() -> anyhow::Result<AnthropicApiKey> {
  let path = credential_path("anthropic_api_key.txt");
  let raw = std::fs::read_to_string(&path).map_err(|e| {
    anyhow::anyhow!("failed to read {} ({e}) — drop your key there for live tests", path.display())
  })?;
  let trimmed = raw.trim();
  if trimmed.is_empty() {
    anyhow::bail!("credential file {} is empty", path.display());
  }
  Ok(AnthropicApiKey::new(trimmed))
}

#[cfg(test)]
fn credential_path(file: &str) -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("../../../external/credentials")
    .join(file)
}
