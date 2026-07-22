//! Typed wire enums for the Veo video endpoints. Each enum exposes `as_str()`
//! (the exact wire string) plus a custom `Serialize` so request structs
//! round-trip cleanly through any log/audit pipeline.

pub mod veo_aspect_ratio;
pub mod veo_duration;
pub mod veo_image_input;
pub mod veo_lite_resolution;
pub mod veo_resolution;
