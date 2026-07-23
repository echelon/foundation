use google_gemini_api_client::api::requests::videos::generate_veo_3_1_lite::generate_veo_3_1_lite::Veo31LiteRequest;
use google_gemini_api_client::api::types::video_types::veo_lite_resolution::VeoLiteResolution;

use crate::error::RouterError;
use crate::image::adjustments::ParamAdjustment;
use crate::video::build::shared::*;
use crate::video::plan::{RouterVideoResolution, VideoGenerationPlan};
use crate::video::route::{BuiltVideoRoute, GoogleVideoSlices, VideoRoute};

pub(crate) fn build(plan: VideoGenerationPlan) -> Result<BuiltVideoRoute, RouterError> {
  let mut adjustments: Vec<ParamAdjustment> = Vec::new();
  let n = clamp_n(&plan, &mut adjustments);
  let resolution = plan.resolution.map(|r| match r {
    RouterVideoResolution::P720 => VeoLiteResolution::P720,
    RouterVideoResolution::P1080 => VeoLiteResolution::P1080,
    RouterVideoResolution::K4 => {
      adjustments.push(ParamAdjustment::coerced("resolution", "Veo 3.1 Lite has no 4K tier — coerced to 1080p"));
      VeoLiteResolution::P1080
    }
  });
  let needs_8s = needs_8s(plan.resolution) || matches!(plan.resolution, Some(RouterVideoResolution::K4));
  let duration = map_duration(plan.duration, needs_8s, &mut adjustments);
  let aspect_ratio = map_aspect(plan.aspect_ratio);
  let person_generation = person_generation_for(&plan);

  // The i2v conditioning image stays as OUR CDN url on the route wrapper —
  // dispatch uploads it to Google's Files API and injects the ref per slice.
  let requests = (0..n)
    .map(|_| Veo31LiteRequest {
      prompt: plan.prompt.clone(),
      negative_prompt: plan.negative_prompt.clone(),
      image: None,
      last_frame: None,
      reference_images: vec![],
      video_uri: None,
      person_generation,
      aspect_ratio,
      resolution,
      duration,
    })
    .collect();
  Ok(BuiltVideoRoute {
    route: VideoRoute::GoogleVeo3_1Lite(GoogleVideoSlices { requests, input_url: plan.image_url.clone() }),
    adjustments,
  })
}
