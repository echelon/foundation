//! Shared video build helpers: validation, `n` clamping, and the common
//! Gemini/Veo param mappings (all three Veo tiers take the same knobs; only
//! the resolution ceiling differs).

use google_gemini_api_client::api::types::video_types::veo_aspect_ratio::VeoAspectRatio;
use google_gemini_api_client::api::types::video_types::veo_duration::VeoDuration;
use google_gemini_api_client::api::types::video_types::veo_person_generation::VeoPersonGeneration;

use crate::error::RouterError;
use crate::image::adjustments::ParamAdjustment;
use crate::video::plan::{RouterVideoAspectRatio, RouterVideoDuration, RouterVideoResolution, VideoGenerationPlan};

pub(crate) const MAX_N: u8 = 10;

pub(crate) fn validate_common(plan: &VideoGenerationPlan) -> Result<(), RouterError> {
  if plan.prompt.trim().is_empty() {
    return Err(RouterError::Invalid("prompt must not be empty".into()));
  }
  if plan.n == 0 {
    return Err(RouterError::Invalid("n must be at least 1".into()));
  }
  if let Some(url) = &plan.image_url {
    if !url.starts_with("https://") && !url.starts_with("http://") {
      return Err(RouterError::Invalid(format!("input image must be an http(s) URL, got: {url}")));
    }
  }
  Ok(())
}

pub(crate) fn clamp_n(plan: &VideoGenerationPlan, adjustments: &mut Vec<ParamAdjustment>) -> u8 {
  if plan.n > MAX_N {
    adjustments.push(ParamAdjustment::coerced("n", format!("{} exceeds the maximum; clamped to {MAX_N}", plan.n)));
    MAX_N
  } else {
    plan.n
  }
}

pub(crate) fn map_aspect(aspect: Option<RouterVideoAspectRatio>) -> Option<VeoAspectRatio> {
  aspect.map(|a| match a {
    RouterVideoAspectRatio::Landscape16x9 => VeoAspectRatio::Landscape16x9,
    RouterVideoAspectRatio::Portrait9x16 => VeoAspectRatio::Portrait9x16,
  })
}

/// Map duration, COERCING to 8 s when the resolution demands it (Veo requires
/// 8 s for 1080p/4K).
pub(crate) fn map_duration(
  duration: Option<RouterVideoDuration>,
  needs_8s: bool,
  adjustments: &mut Vec<ParamAdjustment>,
) -> Option<VeoDuration> {
  let mapped = duration.map(|d| match d {
    RouterVideoDuration::S4 => VeoDuration::S4,
    RouterVideoDuration::S6 => VeoDuration::S6,
    RouterVideoDuration::S8 => VeoDuration::S8,
  });
  if needs_8s && matches!(mapped, Some(VeoDuration::S4) | Some(VeoDuration::S6)) {
    adjustments.push(ParamAdjustment::coerced("duration_seconds", "1080p/4K require 8 s — coerced"));
    return Some(VeoDuration::S8);
  }
  mapped
}

/// Veo's people policy is task-gated: image-to-video permits only allow_adult.
/// Set it automatically for i2v so the provider doesn't reject the call.
pub(crate) fn person_generation_for(plan: &VideoGenerationPlan) -> Option<VeoPersonGeneration> {
  plan.image_url.as_ref().map(|_| VeoPersonGeneration::AllowAdult)
}

pub(crate) fn needs_8s(resolution: Option<RouterVideoResolution>) -> bool {
  matches!(resolution, Some(RouterVideoResolution::P1080) | Some(RouterVideoResolution::K4))
}
