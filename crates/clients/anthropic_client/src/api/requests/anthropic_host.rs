/// Base URL for the Anthropic API. Every request is `{ANTHROPIC_API_BASE_URL}/v1/...`.
pub(crate) const ANTHROPIC_API_BASE_URL: &str = "https://api.anthropic.com";

/// The `anthropic-version` header value pinned by this client.
pub(crate) const ANTHROPIC_VERSION: &str = "2023-06-01";
