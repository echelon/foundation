/// How a provider consumes media inputs — the Kinovi-vs-OpenAI split from artcraft_router,
/// expressed as a capability instead of a Draft type (no upload-first provider is wired
/// yet; when one lands, its build fn grows an async finalize step gated on this).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum InputStrategy {
  /// The provider fetches publicly accessible URLs — we hand it OUR CDN urls directly.
  PublicUrls,
  /// The provider requires assets uploaded to ITS storage first (an async pre-step at
  /// dispatch time).
  ProviderUpload,
}

/// OpenAI fetches public URLs; Google (Gemini API) CANNOT — its inputs go
/// upload-first through the provider Files API at dispatch time.
pub fn input_strategy_for(provider: catalog::Provider) -> InputStrategy {
  match provider {
    catalog::Provider::Openai => InputStrategy::PublicUrls,
    catalog::Provider::Google => InputStrategy::ProviderUpload,
    catalog::Provider::Kinovi => InputStrategy::ProviderUpload,
  }
}
