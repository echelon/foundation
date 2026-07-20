/// Holds the Anthropic API key. Sent on every request as the `x-api-key` header.
#[derive(Clone)]
pub struct AnthropicApiKey {
  pub(crate) api_key: String,
}

impl AnthropicApiKey {
  pub fn new(api_key: impl Into<String>) -> Self {
    Self { api_key: api_key.into() }
  }
}

// Debug is redacted so the key can't leak via `{:?}`.
impl std::fmt::Debug for AnthropicApiKey {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "AnthropicApiKey(<redacted>)")
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn debug_redacts() {
    let dbg = format!("{:?}", AnthropicApiKey::new("sk-ant-secret"));
    assert!(!dbg.contains("sk-ant-secret"));
    assert!(dbg.contains("<redacted>"));
  }
}
