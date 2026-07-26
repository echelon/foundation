//! Kling (Kuaishou) client — Kling video / image / audio generation, plus the
//! element and voice management APIs that feed them.
//!
//! Kling's current API standard ("API 2.0": model version in the URL path, no
//! `model_name` body field) is **async create → poll, with optional callbacks**:
//!
//! | Binding family                       | Mode          | Handle → result                                        |
//! | ------------------------------------ | ------------- | ------------------------------------------------------ |
//! | `video/*`, `image/*`, `audio/*`      | create → poll | task `id` → [`api::requests::tasks::get_tasks`]        |
//! | `elements/*`, `voices/*` (create)    | create → poll | legacy `task_id` → per-family query binding            |
//! | `elements/*`, `voices/*` (query/del) | synchronous   | immediate response                                     |
//!
//! Every create accepts an optional `callback_url` (Kling POSTs the full task
//! payload on status changes) and an `external_task_id` for correlation. Kling
//! callbacks are **unsigned** — treat them as hints and confirm by poll before
//! any state transition (see [`api::webhooks`]). Polling helpers live in
//! [`api::requests::tasks`], including an [`api::requests::tasks::await_task`]
//! loop.
//!
//! Auth is `Authorization: Bearer <api-key>` on every request (see
//! [`creds::kling_api_key`]); the base host is `api-singapore.klingai.com`.
//! Generated assets are purged upstream after 30 days — ingest promptly.
//!
//! Cost estimation (micro-USD, from the official pricing tables) is implemented
//! on each generation request via [`client_core::CostCalculator`].

// The `{binding}/{binding}.rs` module layout is the house style for provider
// clients; the inception lint disagrees; the layout wins.
#![allow(clippy::module_inception)]

// Public API
pub mod api;
pub mod creds;
pub mod error;

#[cfg(test)]
pub(crate) mod test_utils;
