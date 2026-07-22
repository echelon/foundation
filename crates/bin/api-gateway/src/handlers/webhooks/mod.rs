//! Inbound provider webhooks (`/v1/webhooks/{provider}`): signature-verified doors that
//! log every delivery and nudge the matching job forward. The poll loop stays the safety
//! net — a lost or failed webhook never strands a job.

mod openai_webhook_handler;

pub use openai_webhook_handler::*;
