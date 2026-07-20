/// Base URL for the OpenAI API. Every request is `{OPENAI_API_BASE_URL}/v1/...`.
pub(crate) const OPENAI_API_BASE_URL: &str = "https://api.openai.com";

/// Apply auth headers (Bearer + optional org/project) to a request builder.
pub(crate) fn with_auth(
  builder: reqwest::RequestBuilder,
  key: &crate::creds::openai_api_key::OpenAiApiKey,
) -> reqwest::RequestBuilder {
  let mut b = builder.header("Authorization", format!("Bearer {}", key.api_key));
  if let Some(org) = &key.organization {
    b = b.header("OpenAI-Organization", org);
  }
  if let Some(project) = &key.project {
    b = b.header("OpenAI-Project", project);
  }
  b
}
