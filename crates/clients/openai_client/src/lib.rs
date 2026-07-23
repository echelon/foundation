//! OpenAI API client — GPT Image (1 / 1 Mini / 1.5 / 2) and LLMs (GPT-5.6 Sol / 5.5 /
//! 5.4 Nano), all through the **Responses API in background mode**: every binding POSTs
//! `/v1/responses` with `background: true` and returns a response id immediately — nothing
//! is ever held open. Poll with `get_response` (or the `await_response` loop) until
//! terminal; results carry decoded images / output text plus the authoritative `usage`.
//!
//! Design (mirrors the other Foundation provider clients):
//!   - one binding fn PER MODEL, each with its own args struct (`{ api_key, request }` —
//!     the key rides separately so the request alone feeds the cost estimator);
//!   - a `cost.rs` `BetterCostEstimator` per binding (published rates; estimates are labeled);
//!   - strongly typed params — real enums, no stringly-typed values;
//!   - the three-layer error module (client / specific API / generic API);
//!   - image EDIT inputs are public URLs (preferred) or Files-API uploads
//!     (`upload_image_file`) — never inline base64;
//!   - `#[ignore]`d live tests read `external/credentials/openai_api_key.txt`.

// The `{binding}/{binding}.rs` module layout is the house style for provider clients
// (see the seedance client) — the inception lint disagrees; the layout wins.
#![allow(clippy::module_inception)]

pub mod api;
pub mod creds;
pub mod error;

#[cfg(test)]
pub(crate) mod test_utils;
