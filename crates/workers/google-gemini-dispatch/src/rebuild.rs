//! Rebuild a claimed job's route from its persisted (already-sanitized) params.
//! A rebuild failure is PERMANENT — retrying identical inputs cannot help.

use foundation_router::image::build::build_image_route;
use foundation_router::image::route::ImageRoute;
use foundation_router::image::stored::StoredImageParams;
use foundation_router::video::build::build_video_route;
use foundation_router::video::route::VideoRoute;
use foundation_router::video::stored::StoredVideoParams;

pub fn rebuild_image_route(job: &db_queries::Job) -> Result<ImageRoute, String> {
  let params: StoredImageParams =
    serde_json::from_value(job.request_params.clone()).map_err(|e| format!("stored params unreadable: {e}"))?;
  let plan = params.to_plan().ok_or_else(|| format!("model no longer routed: {}", params.model))?;
  let built = build_image_route(plan).map_err(|e| format!("route rebuild failed: {e}"))?;
  Ok(built.route)
}

pub fn rebuild_video_route(job: &db_queries::Job) -> Result<VideoRoute, String> {
  let params: StoredVideoParams =
    serde_json::from_value(job.request_params.clone()).map_err(|e| format!("stored params unreadable: {e}"))?;
  let plan = params.to_plan().ok_or_else(|| format!("model no longer routed: {}", params.model))?;
  let built = build_video_route(plan).map_err(|e| format!("route rebuild failed: {e}"))?;
  Ok(built.route)
}
