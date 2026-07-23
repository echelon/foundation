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
//!   - `#[ignore]`d live tests read `external/credentials/google_gemini_api_key.txt`.
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
//!
//! # Connection model (per binding, live-verified)
//!
//! | Binding                        | Mode  | Handle → poll                                     |
//! |--------------------------------|-------|---------------------------------------------------|
//! | `generate_nano_banana*` (×4)   | SYNC  | none — connection held ~5–40 s (no async offered) |
//! | `generate_veo_3_1{,_fast,_lite}` | ASYNC | `VeoOperationName` → `poll_video_operation` (~10 s) |
//! | `generate_gemini_omni_flash`   | ASYNC | `InteractionHandle` → `get_interaction` (~5 s)     |
//! | `generate_lyria_3_{clip,pro}`  | ASYNC | `InteractionHandle` → `get_interaction` (~5 s)     |
//! | `chat_gemini_3_6_flash`        | ASYNC | `InteractionHandle` → `get_interaction` (~3 s)     |
//! | `chat_gemini_3_5_flash_lite`   | SYNC  | none — Google gates it out of background mode      |
//! | files upload/get/download      | SYNC  | fast administrative calls                          |
//!
//! Every async handle is a durable server-side resource: safe across process
//! restarts, and the natural value for a dispatcher's `provider_job_id`.
//! Responses carry provider-reported token usage where the API returns it —
//! the authoritative numbers behind every `Estimated` cost.

#[cfg(test)]
pub(crate) mod test_utils;

// Public API
pub mod api;
pub mod creds;
pub mod error;
