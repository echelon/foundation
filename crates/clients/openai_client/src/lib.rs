//! OpenAI API client — GPT Image (1 / 1 Mini / 1.5 / 2) and LLMs (GPT-5.6 Sol / 5.5 /
//! 5.4 Nano), all through the **Responses API in background mode**: every binding POSTs
//! `/v1/responses` with `background: true` and returns a response id immediately — nothing
//! is ever held open. Poll with `get_response` (or the `await_response` loop) until
//! terminal; results carry decoded images / output text plus the authoritative `usage`.
//!
//! Design (mirrors the other Foundation provider clients):
//!   - one binding fn PER MODEL, each with its own args struct (`{ api_key, request }` —
//!     the key rides separately so the request alone feeds the cost estimator);
//!   - a `cost.rs` `CostCalculator` per binding (published rates; estimates are labeled);
//!   - strongly typed params — real enums, no stringly-typed values;
//!   - the three-layer error module (client / specific API / generic API);
//!   - image EDIT inputs are public URLs (preferred) or Files-API uploads
//!     (`upload_image_file`) — never inline base64;
//!   - `#[ignore]`d live tests read `external/credentials/openai_api_key.txt`.
//!
//! # Connection model (per binding)
//!
//! | Binding                         | Mode  | Handle → poll                                  |
//! |---------------------------------|-------|-------------------------------------------------|
//! | `generate_gpt_image_{1,1_mini,1_5,2}` | ASYNC | response id → `get_response` / `await_response` |
//! | `chat_gpt_{5_5,5_6_sol,5_4_nano}`     | ASYNC | response id → `get_response` / `await_response` |
//! | `upload_image_file`             | SYNC  | fast administrative call                        |
//!
//! Response ids are durable server-side resources (restart-safe; the natural
//! `provider_job_id`), and OpenAI additionally delivers signed webhooks for
//! background responses (`api::webhooks`) — the poll loop remains the safety net.
//!
//! # Parameter coverage (audited 2026-07 against the Responses API reference)
//!
//! Bound on chat: messages/roles, `max_output_tokens`, `temperature`, `top_p`,
//! `user`, `reasoning.effort`, structured output (`text.format` json_schema),
//! `previous_response_id`. Bound on the image tool: `size`, `quality`,
//! `output_format`, `output_compression`, `background` (transparency; rejected
//! client-side for gpt-image-2), `moderation`, `input_fidelity` (not on
//! gpt-image-2), `input_image_mask`, `action`.
//!
//! Deliberately NOT bound (would change the binding surface; add when needed):
//! function/web-search/file-search/MCP tools on chat (the poll layer has no
//! tool-call loop yet), `instructions` (system messages cover it), `metadata`,
//! `service_tier`, `truncation`, `include`, streaming/`partial_images` (we
//! poll, never stream), and `stream`.

// The `{binding}/{binding}.rs` module layout is the house style for provider clients
// (see the seedance client) — the inception lint disagrees; the layout wins.
#![allow(clippy::module_inception)]

pub mod api;
pub mod creds;
pub mod error;

#[cfg(test)]
pub(crate) mod test_utils;
