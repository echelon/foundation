//! Generation: enqueue (jobs.md admission), the embedded dispatcher (dispatch.md's worker
//! loop), and job response shaping. The API layer NEVER calls a provider — enqueue commits
//! a queued job; the dispatcher fires and polls in the background.

pub mod dispatcher;
pub mod job_response;
pub mod service;
pub mod stored_params;
