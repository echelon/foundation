//! Voice Management (`/v1/general/custom-voices`, `presets-voices`) — custom
//! voices usable by Kling 2.6 image-to-video voice control and bindable to
//! elements (Kling 3.0 Omni). Kling documents this same endpoint set on two
//! doc pages (video 3.0 Omni, video 2.6) — it is ONE service, so this crate
//! binds it once.
//!
//! CRUD surface: create (async task, legacy vocabulary), query single, query
//! custom list, query presets list, delete. There is no update endpoint.

pub mod create_voice;
pub mod delete_voice;
pub mod get_voice;
pub mod list_preset_voices;
pub mod list_voices;
