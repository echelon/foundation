//! New-style task querying (`GET /tasks`, `POST /tasks`) and the poll loop.
//!
//! These serve every model-version-in-path create binding (video, omni,
//! motion control). A single task is fetched with
//! [`get_tasks::get_tasks`] and a one-element
//! [`get_tasks::TaskQuery::ByTaskIds`]; [`await_task::await_task`] wraps that
//! in a sleep loop. Legacy `/v1/*` families (image, audio, elements, voices)
//! have their own per-family query bindings instead.

pub mod await_task;
pub mod get_tasks;
pub mod list_tasks;
