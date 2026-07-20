//! Anthropic API client — Claude LLM via the Messages API (`POST /v1/messages`).
//!
//! Same layered organization as the other Foundation provider clients: a three-layer error
//! module, redacted credentials, typed request/response bindings with a `cost.rs` cost
//! estimator, typed model/role enums, and unit tests.

pub mod api;
pub mod creds;
pub mod error;

#[cfg(test)]
pub(crate) mod test_utils;
