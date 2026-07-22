//! Google AI Studio (Gemini API) client — the **Nano Banana** image family and
//! the **Veo 3.1** video family, sourced DIRECTLY from Google
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
//!   - `#[ignore]`d live tests read `external/credentials/google_aistudio_api_key.txt`.
//!
//! Async posture: **Veo is natively async** (`predictLongRunning` → poll →
//! download; results persist ~2 days). **Image generation is synchronous by
//! provider design** — no poll/webhook variant exists for interactive use (the
//! Batch API is 24-hour-turnaround bulk only), so run image calls from a
//! worker, never a held-open handler.
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
