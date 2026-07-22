use openai_client::api::requests::images::generate_gpt_image_1_5::generate_gpt_image_1_5::GptImage1_5Request;

use crate::error::RouterError;
use crate::image::adjustments::ParamAdjustment;
use crate::image::build::shared::*;
use crate::image::plan::ImageGenerationPlan;
use crate::image::route::{BuiltImageRoute, ImageRoute};

/// Build the gpt-image-1.5 route: prune/coerce, then fan out `n` identical provider requests
/// (one per slice — each becomes its own background response).
pub(crate) fn build(plan: ImageGenerationPlan) -> Result<BuiltImageRoute, RouterError> {
  let mut adjustments: Vec<ParamAdjustment> = Vec::new();
  let n = clamp_n(&plan, &mut adjustments);

  let mut requests = Vec::with_capacity(n as usize);
  for _ in 0..n {
    let mut request = GptImage1_5Request::new(plan.prompt.clone());
    request.input_images = input_sources(&plan);
    request.mask = mask_source(&plan);
    request.size = plan.size.map(map_size);
    request.quality = plan.quality.map(map_quality);
    request.output_format = plan.output_format.map(map_format);
    request.input_fidelity = plan.input_fidelity.map(map_fidelity);
    requests.push(request);
  }
  Ok(BuiltImageRoute { route: ImageRoute::OpenaiGptImage1_5(requests), adjustments })
}
