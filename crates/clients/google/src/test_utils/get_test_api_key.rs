use std::fs::read_to_string;

use crate::creds::gemini_api_key::GeminiApiKey;

const GEMINI_API_KEY_PATH: &str = "/Users/bt/Artcraft/credentials/gemini_api_key.txt";

/// Load a Google Gemini API key from the local credentials file. Whitespace is
/// trimmed because key files often pick up a trailing newline.
#[cfg(test)]
pub fn get_test_api_key() -> anyhow::Result<GeminiApiKey> {
  let raw = read_to_string(GEMINI_API_KEY_PATH)?;
  let trimmed = raw.trim().to_string();
  if trimmed.is_empty() {
    anyhow::bail!("Gemini API key file is empty: {}", GEMINI_API_KEY_PATH);
  }
  Ok(GeminiApiKey::new(trimmed))
}
