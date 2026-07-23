//! Per-model video build fns (mirrors `image::build`).

mod google_veo_3_1;
mod google_veo_3_1_fast;
mod google_veo_3_1_lite;
pub(crate) mod shared;

use crate::error::RouterError;
use crate::video::plan::{RouterVideoModel, VideoGenerationPlan};
use crate::video::route::BuiltVideoRoute;

/// THE video router entry point: hard-coded dispatch by model.
pub fn build_video_route(plan: VideoGenerationPlan) -> Result<BuiltVideoRoute, RouterError> {
  shared::validate_common(&plan)?;
  match plan.model {
    RouterVideoModel::GoogleVeo3_1 => google_veo_3_1::build(plan),
    RouterVideoModel::GoogleVeo3_1Fast => google_veo_3_1_fast::build(plan),
    RouterVideoModel::GoogleVeo3_1Lite => google_veo_3_1_lite::build(plan),
  }
}
