//! OpenAI API client — GPT Image (`/v1/images/*`) and chat/LLM (`/v1/chat/completions`).
//!
//! Organized like ArtCraft's clients but sharing `client-core` plumbing: a layered error
//! module, redacted credentials, typed request/response bindings per endpoint (each with a
//! `cost.rs` cost calculator), typed model/param enums, and thorough unit tests.

pub mod api;
pub mod creds;
pub mod error;

#[cfg(test)]
pub(crate) mod test_utils;
