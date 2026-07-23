//! Generation endpoints: the Any plane (`/v1/any/image`), the unified model plane
//! (`/v1/models/{slug}/generate`), cost twins, and job polling/listing.

pub mod types;
pub mod video_types;

mod admin_get_job_request_handler;
mod admin_list_jobs_handler;
mod cost_any_image_handler;
mod cost_any_video_handler;
mod cost_with_model_handler;
mod generate_any_image_handler;
mod generate_any_video_handler;
mod generate_with_model_handler;
mod get_job_handler;
mod get_job_request_handler;
mod get_job_status_handler;
mod list_jobs_handler;

pub use admin_get_job_request_handler::*;
pub use admin_list_jobs_handler::*;
pub use cost_any_image_handler::*;
pub use cost_any_video_handler::*;
pub use cost_with_model_handler::*;
pub use generate_any_image_handler::*;
pub use generate_any_video_handler::*;
pub use generate_with_model_handler::*;
pub use get_job_handler::*;
pub use get_job_request_handler::*;
pub use get_job_status_handler::*;
pub use list_jobs_handler::*;
