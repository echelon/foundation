use google_gemini_api_client::api::requests::images::generate_nano_banana_2::generate_nano_banana_2::NanoBanana2Request;

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
  use google_gemini_api_client::api::types::image_types::nano_banana_2_image_size::NanoBanana2ImageSize;
  let image_size = match plan.quality {
    None | Some(crate::image::plan::RouterImageQuality::Auto) => None,
    Some(crate::image::plan::RouterImageQuality::Low) => {
      adjustments.push(ParamAdjustment::coerced("quality", "low → image_size 1K"));
      Some(NanoBanana2ImageSize::K1)
    }
    Some(crate::image::plan::RouterImageQuality::Medium) => {
      adjustments.push(ParamAdjustment::coerced("quality", "medium → image_size 2K"));
      Some(NanoBanana2ImageSize::K2)
    }
    Some(crate::image::plan::RouterImageQuality::High) => {
      adjustments.push(ParamAdjustment::coerced("quality", "high → image_size 4K"));
      Some(NanoBanana2ImageSize::K4)
    }
  };
  prune_unsupported(&plan, &mut adjustments);

  // Input images stay as OUR CDN urls on the route wrapper — dispatch uploads
  // them to Google's Files API and injects the refs (ProviderUpload strategy).
  let requests = (0..n)
    .map(|_| NanoBanana2Request {
      prompt: plan.prompt.clone(),
      images: vec![],
      aspect_ratio,
    image_size,
    })
    .collect();
  Ok(BuiltImageRoute {
    route: ImageRoute::GoogleNanoBanana2(GoogleImageSlices { requests, input_urls: plan.image_urls.clone() }),
    adjustments,
  })
}
