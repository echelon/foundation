//! Gemini API client (keys minted in Google AI Studio) — the **Nano Banana** image family,
//! the **Veo 3.1** video family, **Lyria 3** music, and **Gemini Omni Flash**
//! multimodal video, sourced DIRECTLY from Google
//! (`generativelanguage.googleapis.com`, `x-goog-api-key` auth).
//!
//! Shape (mirrors `openai_client` / `kinovi_client`):
//!   - one binding fn per model (`generate_nano_banana_2`, `generate_veo_3_1`, …),
//!     each pinning its model id; args structs carry the API key SEPARATELY from
//!     the request so the request alone drives pricing;
//!   - a [`client_core::CostCalculator`] impl (+ certainty) next to each binding
//!     in its `cost.rs`;
//!   - three-layer error taxonomy (client / api-specific / api-generic);
//!   - typed wire enums — no stringly-typed values;
//!   - `#[ignore]`d live tests read `external/credentials/gemini_api_key.txt`.
//!
//! Async posture: **Veo is natively async** (`predictLongRunning` → poll →
//! download; results persist ~2 days). **Image, music (Lyria), and Omni calls
//! are synchronous by provider design** — run them from a worker, never a
//! held-open handler. Omni softens this with `delivery: uri` (this crate's
//! default): the response is a Files handle to poll + download rather than a
//! giant inline body.
//!
//! Input images: the Gemini API does NOT fetch public HTTP URLs. Prefer the
//! free Files API (`api::requests::files::upload_image_file`, ~48 h retention)
//! and pass `file_data` references; inline base64 exists as a fallback only.

#[cfg(test)]
pub(crate) mod test_utils;

// Public API
pub mod api;
pub mod creds;
pub mod error;
