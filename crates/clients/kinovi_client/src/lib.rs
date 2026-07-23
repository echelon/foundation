//! Kinovi (seedance2pro) client — ByteDance **Seedance 2.0 / Fast / Mini** video.
//!
//! Kinovi is an async, poll-based video provider. The two-call flow:
//!
//! 1. [`api::requests::video::create_task`] — `POST /jobs/createTask` with a typed
//!    [`api::requests::video::create_task::create_task::CreateTaskRequest`]. Returns a
//!    provider `taskId`.
//! 2. [`api::requests::video::poll_task`] — `GET /jobs/recordInfo?taskId=…` until the task's
//!    [`client_core::JobState`] is terminal. On success the video URL is exposed as a
//!    [`client_core::Asset::Url`].
//!
//! Auth is `Authorization: Bearer <key>` on every request (see [`creds::kinovi_api_key`]).
//! The rich, provider-specific error taxonomy lives in [`error`]; cost estimation (per
//! video-second, in `money::MicroUsd`) is implemented on the create-task request via
//! [`client_core::CostCalculator`].

// Public API
pub mod api;
pub mod creds;
pub mod error;

#[cfg(test)]
pub(crate) mod test_utils;
