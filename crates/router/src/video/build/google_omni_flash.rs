//! Gemini Omni Flash (`gemini-omni-flash-preview`) — the multimodal video model
//! on the Interactions API. Its knobs differ from Veo's: output length is
//! prompt-driven (no duration param), there is no resolution tier and no
//! discrete negative prompt — the superset params are pruned with notes, and a
//! requested duration still informs the cost ESTIMATE (Omni is token-billed).

use google_gemini_api_client::api::requests::videos::generate_gemini_omni_flash::generate_gemini_omni_flash::GeminiOmniFlashRequest;
use google_gemini_api_client::api::types::video_types::omni_aspect_ratio::OmniAspectRatio;

use crate::error::RouterError;
use crate::image::adjustments::ParamAdjustment;
use crate::video::build::shared::clamp_n;
use crate::video::plan::{RouterVideoAspectRatio, VideoGenerationPlan};
use crate::video::route::{BuiltVideoRoute, GoogleVideoSlices, VideoRoute};

pub(crate) fn build(plan: VideoGenerationPlan) -> Result<BuiltVideoRoute, RouterError> {
  let mut adjustments: Vec<ParamAdjustment> = Vec::new();
  let n = clamp_n(&plan, &mut adjustments);

  // Fold the negative prompt INTO the prompt (Omni has no discrete param).
  let prompt = match plan.negative_prompt.as_deref().map(str::trim) {
    Some(neg) if !neg.is_empty() => {
      adjustments.push(ParamAdjustment::coerced(
        "negative_prompt",
        "Omni has no negative_prompt parameter — folded into the prompt as avoidance guidance",
      ));
      format!("{}\n\nAvoid: {neg}", plan.prompt)
    }
    _ => plan.prompt.clone(),
  };
  if plan.resolution.is_some() {
    adjustments.push(ParamAdjustment::pruned(
      "resolution",
      "Omni has no resolution parameter (~720p output) — pruned",
    ));
  }
  let expected_output_seconds = plan.duration.map(|d| d.seconds());
  if plan.duration.is_some() {
    adjustments.push(ParamAdjustment::pruned(
      "duration_seconds",
      "Omni's output length is prompt-driven — not sent; used only to refine the cost estimate",
    ));
  }
  let aspect_ratio = plan.aspect_ratio.map(|a| match a {
    RouterVideoAspectRatio::Landscape16x9 => OmniAspectRatio::Landscape16x9,
    RouterVideoAspectRatio::Portrait9x16 => OmniAspectRatio::Portrait9x16,
  });

  // The i2v conditioning image stays as OUR CDN url on the route wrapper —
  // dispatch uploads it to Google's Files API and injects the ref per slice
  // (the Interactions API takes Files-API document items).
  let requests = (0..n)
    .map(|_| GeminiOmniFlashRequest {
      prompt: prompt.clone(),
      images: vec![],
      video_file_uri: None,
      task: None, // server infers text_to_video vs reference_to_video from the inputs
      aspect_ratio,
      delivery: None, // binding default: URI delivery (inline only fits ≤ 4 MB)
      previous_interaction_id: None,
      store: None,
      expected_output_seconds,
    })
    .collect();
  Ok(BuiltVideoRoute {
    route: VideoRoute::GoogleOmniFlash(GoogleVideoSlices { requests, input_url: plan.image_url.clone() }),
    adjustments,
  })
}
