//! Element Management (`/v1/general/advanced-*`) — custom characters/objects
//! usable by Kling 3.0 / 3.0 Omni video (both element kinds), Kling O1 video
//! (multi-image elements only), and Kling Image 3.0 Omni / Image O1
//! (`element_list`). Kling documents this same endpoint set on four doc
//! pages (video 3.0/Omni, video O1, image 3.0/Omni, image O1) — it is ONE
//! service, so this crate binds it once.
//!
//! CRUD surface: create (async task, legacy vocabulary), query single, query
//! custom list, query presets list, delete. There is no update endpoint.

pub mod create_element;
pub mod delete_element;
pub mod get_element;
pub mod list_elements;
pub mod list_preset_elements;
