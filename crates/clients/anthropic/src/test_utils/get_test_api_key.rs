use crate::creds::anthropic_api_key::AnthropicApiKey;

/// Load an Anthropic API key for `#[ignore]` live tests from `ANTHROPIC_API_KEY`.
#[cfg(test)]
pub fn get_test_api_key() -> anyhow::Result<AnthropicApiKey> {
  let raw = std::env::var("ANTHROPIC_API_KEY")
    .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY is not set (required for live tests)"))?;
  let trimmed = raw.trim();
  if trimmed.is_empty() {
    anyhow::bail!("ANTHROPIC_API_KEY is empty");
  }
  Ok(AnthropicApiKey::new(trimmed))
}
