//! The Google Gemini dispatch flows, shared by the two worker binaries:
//!
//! - `google-gemini-outbound-job` — claims queued `provider = 'google'` jobs and
//!   RUNS them: Nano Banana synchronously to completion, Veo by starting the
//!   provider operation (`image_dispatch` / `video_dispatch`).
//! - `google-gemini-polling-job` — drives started Veo operations to completion
//!   (`video_poll_flow`).
//!
//! One module per seam; the binaries are thin loops around these flows.

pub mod complete;
pub mod context;
pub mod image_dispatch;
pub mod ingest;
pub mod inputs;
pub mod provider_log;
pub mod rebuild;
pub mod video_dispatch;
pub mod video_poll_flow;
