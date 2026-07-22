//! Typed wire enums for the Gemini image endpoints (aspect ratio, per-model
//! size tiers, input images). Each enum exposes `as_str()` (the exact wire
//! string) plus a custom `Serialize` so request structs round-trip cleanly
//! through any log/audit pipeline.

pub mod image_aspect_ratio;
pub mod image_input;
pub mod nano_banana_2_image_size;
pub mod nano_banana_pro_image_size;
