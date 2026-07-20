//! OpenAI API client — GPT Image (`/v1/images/*`) and chat/LLM (`/v1/chat/completions`).
//!
//! Layered like the other Foundation provider clients, sharing `client-core` plumbing: a
//! three-layer error module, redacted credentials, typed request/response bindings per
//! endpoint (each with a `cost.rs` cost calculator), typed model/param enums, and unit tests.

pub mod api;
pub mod creds;
pub mod error;

#[cfg(test)]
pub(crate) mod test_utils;
