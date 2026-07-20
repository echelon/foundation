//! Google Gemini image client — the **Nano Banana** family
//! (`gemini-2.5-flash-image`, `gemini-3-pro-image`, `gemini-3.1-flash-image`).
//!
//! `POST /v1beta/models/{model}:generateContent`, auth via the `x-goog-api-key`
//! header. Synchronous: image bytes come back base64-encoded in
//! `candidates[].content.parts[].inlineData` and are decoded into
//! [`client_core::Asset::Bytes`].
//!
//! Structure mirrors the ArtCraft provider clients: a rich three-layer error
//! module, redacted-`Debug` credentials, typed wire enums with `as_str()` +
//! custom `Serialize`, per-endpoint directories, and a [`client_core::CostCalculator`]
//! impl next to each endpoint in its `cost.rs`.

#[cfg(test)]
pub(crate) mod test_utils;

// Public API
pub mod api;
pub mod creds;
pub mod error;
