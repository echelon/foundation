//! Build helpers shared by the Google Nano Banana family. The Gemini API's
//! image knobs differ from the plan's GPT-flavored superset, so the family
//! maps/coerces heavily and RECORDS every change:
//!   - `size` (WxH) → the nearest `aspect_ratio` (coerced);
//!   - `quality` → a resolution tier on models that have tiers, pruned elsewhere;
//!   - `output_format` → pruned (the provider picks the mime);
//!   - `input_fidelity`, `mask` → pruned (no Gemini equivalent);
//!   - input images stay as OUR CDN urls on the route — Gemini cannot fetch
//!     public URLs, so dispatch uploads them to the provider's Files API first
//!     (`InputStrategy::ProviderUpload`).

use google_gemini_api_client::api::types::image_types::image_aspect_ratio::ImageAspectRatio;

use crate::image::adjustments::ParamAdjustment;
use crate::image::plan::{ImageGenerationPlan, RouterImageSize};

/// `size` → the aspect ratio that preserves the requested framing.
pub(crate) fn size_to_aspect(
  size: Option<RouterImageSize>,
  adjustments: &mut Vec<ParamAdjustment>,
) -> Option<ImageAspectRatio> {
  match size? {
    RouterImageSize::Auto => None,
    RouterImageSize::Square1024 => {
      adjustments.push(ParamAdjustment::coerced("size", "1024x1024 → aspect_ratio 1:1 (Gemini sizes by ratio + tier)"));
      Some(ImageAspectRatio::Square)
    }
    RouterImageSize::Landscape1536x1024 => {
      adjustments.push(ParamAdjustment::coerced("size", "1536x1024 → aspect_ratio 3:2 (Gemini sizes by ratio + tier)"));
      Some(ImageAspectRatio::Landscape3x2)
    }
    RouterImageSize::Portrait1024x1536 => {
      adjustments.push(ParamAdjustment::coerced("size", "1024x1536 → aspect_ratio 2:3 (Gemini sizes by ratio + tier)"));
      Some(ImageAspectRatio::Portrait2x3)
    }
  }
}

/// Prune the params the Nano Banana family has no equivalent for.
pub(crate) fn prune_unsupported(plan: &ImageGenerationPlan, adjustments: &mut Vec<ParamAdjustment>) {
  if plan.output_format.is_some() {
    adjustments.push(ParamAdjustment::pruned("output_format", "Gemini picks the output mime itself"));
  }
  if plan.input_fidelity.is_some() {
    adjustments.push(ParamAdjustment::pruned("input_fidelity", "not supported by the Nano Banana family"));
  }
  if plan.mask_url.is_some() {
    adjustments.push(ParamAdjustment::pruned("mask", "Gemini has no inpainting mask — describe the edit region in the prompt"));
  }
}
