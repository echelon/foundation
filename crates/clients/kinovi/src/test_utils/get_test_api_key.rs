use std::fs::read_to_string;

use crate::creds::kinovi_api_key::KinoviApiKey;

const KINOVI_API_KEY_PATH: &str = "/Users/bt/Artcraft/credentials/kinovi_api_key.txt";

/// Load a Kinovi API key from the local credentials file. Whitespace is trimmed
/// because key files often pick up a trailing newline. Used only by the
/// `#[ignore]`d live tests.
#[cfg(test)]
pub fn get_test_api_key() -> anyhow::Result<KinoviApiKey> {
  let raw = read_to_string(KINOVI_API_KEY_PATH)?;
  let trimmed = raw.trim().to_string();
  if trimmed.is_empty() {
    anyhow::bail!("Kinovi API key file is empty: {}", KINOVI_API_KEY_PATH);
  }
  Ok(KinoviApiKey::new(trimmed))
}
