use std::path::PathBuf;

use crate::creds::openai_api_key::OpenAiApiKey;

/// Load the OpenAI key for `#[ignore]` live tests from
/// `{repo}/external/credentials/openai_api_key.txt` (whitespace trimmed).
#[cfg(test)]
pub fn get_test_api_key() -> anyhow::Result<OpenAiApiKey> {
  let path = credential_path("openai_api_key.txt");
  let raw = std::fs::read_to_string(&path).map_err(|e| {
    anyhow::anyhow!("failed to read {} ({e}) — drop your key there for live tests", path.display())
  })?;
  let trimmed = raw.trim();
  if trimmed.is_empty() {
    anyhow::bail!("credential file {} is empty", path.display());
  }
  Ok(OpenAiApiKey::new(trimmed))
}

/// Resolve a credential file under the repo root (`external/credentials/`). The crate lives at
/// `crates/clients/<crate>`, so the repo root is three levels up from `CARGO_MANIFEST_DIR`.
#[cfg(test)]
fn credential_path(file: &str) -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("../../../external/credentials")
    .join(file)
}
