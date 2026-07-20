use crate::creds::openai_api_key::OpenAiApiKey;

/// Load an OpenAI API key for the `#[ignore]` live tests. Reads `OPENAI_API_KEY` from the
/// environment; whitespace is trimmed. Returns an error (so the test fails loudly) when unset.
#[cfg(test)]
pub fn get_test_api_key() -> anyhow::Result<OpenAiApiKey> {
  let raw = std::env::var("OPENAI_API_KEY")
    .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY is not set (required for live tests)"))?;
  let trimmed = raw.trim();
  if trimmed.is_empty() {
    anyhow::bail!("OPENAI_API_KEY is empty");
  }
  Ok(OpenAiApiKey::new(trimmed))
}
