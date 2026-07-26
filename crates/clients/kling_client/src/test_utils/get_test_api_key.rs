use std::path::PathBuf;

use crate::creds::kling_api_key::KlingApiKey;

/// Load the live-test API key from `external/secrets/kling_api_key.txt`
/// (gitignored). Only ever used by `#[ignore]`d live tests — production code
/// takes the key as a constructor argument.
#[cfg(test)]
pub fn get_test_api_key() -> anyhow::Result<KlingApiKey> {
  let path = secret_path("kling_api_key.txt");
  let raw = std::fs::read_to_string(&path).map_err(|e| {
    anyhow::anyhow!("failed to read {} ({e}) — drop your key there for live tests", path.display())
  })?;
  let trimmed = raw.trim();
  if trimmed.is_empty() {
    anyhow::bail!("credential file {} is empty", path.display());
  }
  Ok(KlingApiKey::new(trimmed))
}

#[cfg(test)]
fn secret_path(file: &str) -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("../../../external/secrets")
    .join(file)
}
