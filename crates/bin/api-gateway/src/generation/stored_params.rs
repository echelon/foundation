//! Re-export of the router's persisted param shapes (`jobs.request_params`). The shapes
//! live in `foundation_router` so the gateway and the worker binaries rebuild from ONE
//! definition; gateway code keeps importing them from here.

pub use foundation_router::image::stored::StoredImageParams;
pub use foundation_router::video::stored::StoredVideoParams;
