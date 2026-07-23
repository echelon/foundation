use google_gemini_api_client::api::requests::images::generate_nano_banana::generate_nano_banana::NanoBananaRequest;

use crate::error::RouterError;
use crate::image::adjustments::ParamAdjustment;
use crate::image::build::google_shared::{prune_unsupported, size_to_aspect};
use crate::image::build::shared::clamp_n;
use crate::image::plan::ImageGenerationPlan;
use crate::image::route::{BuiltImageRoute, GoogleImageSlices, ImageRoute};

pub(crate) fn build(plan: ImageGenerationPlan) -> Result<BuiltImageRoute, RouterError> {
  let mut adjustments: Vec<ParamAdjustment> = Vec::new();
  let n = clamp_n(&plan, &mut adjustments);
  let aspect_ratio = size_to_aspect(plan.size, &mut adjustments);
  if plan.quality.is_some() {
    adjustments.push(ParamAdjustment::pruned("quality", "Nano Banana has one flat tier (1024px)"));
  }
  prune_unsupported(&plan, &mut adjustments);

  // Input images stay as OUR CDN urls on the route wrapper — dispatch uploads
  // them to Google's Files API and injects the refs (ProviderUpload strategy).
  let requests = (0..n)
    .map(|_| NanoBananaRequest {
      prompt: plan.prompt.clone(),
      images: vec![],
      aspect_ratio,
    })
    .collect();
  Ok(BuiltImageRoute {
    route: ImageRoute::GoogleNanoBanana(GoogleImageSlices { requests, input_urls: plan.image_urls.clone() }),
    adjustments,
  })
}
