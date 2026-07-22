//! `job_events` table (jobs.md) — the append-only job trail.

mod create_job_event;

pub use create_job_event::create_job_event;
