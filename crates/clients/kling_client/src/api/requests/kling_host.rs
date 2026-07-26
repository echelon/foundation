use crate::creds::kling_api_key::KlingApiKey;

/// Kling's global (non-China) API host. The old `api.klingai.com` is
/// deprecated; Singapore is the documented endpoint for servers outside China.
pub(crate) const KLING_API_BASE_URL: &str = "https://api-singapore.klingai.com";

pub(crate) const KLING_USER_AGENT: &str = "foundation/kling-api-client";

/// Apply Kling auth to a request: `Authorization: Bearer <api-key>`.
pub(crate) fn with_auth(builder: reqwest::RequestBuilder, key: &KlingApiKey) -> reqwest::RequestBuilder {
  builder.bearer_auth(&key.api_key)
}
