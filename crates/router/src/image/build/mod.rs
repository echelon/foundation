//! Per-model build fns: prune/coerce the plan for the routed model, record adjustments,
//! and fan `n` out into N provider-native requests. One file per model (artcraft layout).

mod openai_gpt_image_1;
mod openai_gpt_image_1_5;
mod openai_gpt_image_1_mini;
mod openai_gpt_image_2;
pub(crate) mod shared;

use crate::error::RouterError;
use crate::image::plan::{ImageGenerationPlan, RouterImageModel};
use crate::image::route::BuiltImageRoute;

/// THE router entry point: hard-coded dispatch by model.
pub fn build_image_route(plan: ImageGenerationPlan) -> Result<BuiltImageRoute, RouterError> {
  shared::validate_common(&plan)?;
  match plan.model {
    RouterImageModel::OpenaiGptImageV1 => openai_gpt_image_1::build(plan),
    RouterImageModel::OpenaiGptImageV1Mini => openai_gpt_image_1_mini::build(plan),
    RouterImageModel::OpenaiGptImageV1_5 => openai_gpt_image_1_5::build(plan),
    RouterImageModel::OpenaiGptImageV2 => openai_gpt_image_2::build(plan),
  }
}
