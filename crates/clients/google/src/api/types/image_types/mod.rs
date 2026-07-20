//! Typed wire enums for the Gemini image endpoint (model, aspect ratio, image
//! size). Each enum exposes `as_str()` (the exact wire string) plus a custom
//! `Serialize` so request structs round-trip cleanly through any log/audit
//! pipeline.

pub mod gemini_image_model;
pub mod image_aspect_ratio;
pub mod image_size;
