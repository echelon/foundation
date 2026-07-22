//! Build helpers shared by the OpenAI GPT Image family (they take the same param set;
//! only `input_fidelity` support differs).

use openai_client::api::types::image_types::image_quality::ImageQuality;
use openai_client::api::types::image_types::image_size::ImageSize;
use openai_client::api::types::image_types::input_fidelity::InputFidelity;
use openai_client::api::types::image_types::input_image_source::InputImageSource;
use openai_client::api::types::image_types::output_format::OutputFormat;

use crate::error::RouterError;
use crate::image::adjustments::ParamAdjustment;
use crate::image::plan::{
  ImageGenerationPlan, RouterImageQuality, RouterImageSize, RouterInputFidelity, RouterOutputFormat,
};

pub(crate) const MAX_N: u8 = 10;

pub(crate) fn validate_common(plan: &ImageGenerationPlan) -> Result<(), RouterError> {
  if plan.prompt.trim().is_empty() {
    return Err(RouterError::Invalid("prompt must not be empty".into()));
  }
  if plan.n == 0 {
    return Err(RouterError::Invalid("n must be at least 1".into()));
  }
  if plan.mask_url.is_some() && plan.image_urls.is_empty() {
    return Err(RouterError::Invalid("a mask requires at least one input image".into()));
  }
  for url in plan.image_urls.iter().chain(plan.mask_url.iter()) {
    if !url.starts_with("https://") && !url.starts_with("http://") {
      return Err(RouterError::Invalid(format!("input image must be an http(s) URL, got: {url}")));
    }
  }
  Ok(())
}

/// Clamp `n` into 1..=10 (recording the coercion).
pub(crate) fn clamp_n(plan: &ImageGenerationPlan, adjustments: &mut Vec<ParamAdjustment>) -> u8 {
  if plan.n > MAX_N {
    adjustments.push(ParamAdjustment::coerced("n", format!("{} exceeds the maximum; clamped to {MAX_N}", plan.n)));
    MAX_N
  } else {
    plan.n
  }
}

pub(crate) fn map_size(size: RouterImageSize) -> ImageSize {
  match size {
    RouterImageSize::Square1024 => ImageSize::Square1024,
    RouterImageSize::Landscape1536x1024 => ImageSize::Landscape1536x1024,
    RouterImageSize::Portrait1024x1536 => ImageSize::Portrait1024x1536,
    RouterImageSize::Auto => ImageSize::Auto,
  }
}

pub(crate) fn map_quality(quality: RouterImageQuality) -> ImageQuality {
  match quality {
    RouterImageQuality::Low => ImageQuality::Low,
    RouterImageQuality::Medium => ImageQuality::Medium,
    RouterImageQuality::High => ImageQuality::High,
    RouterImageQuality::Auto => ImageQuality::Auto,
  }
}

pub(crate) fn map_format(format: RouterOutputFormat) -> OutputFormat {
  match format {
    RouterOutputFormat::Png => OutputFormat::Png,
    RouterOutputFormat::Jpeg => OutputFormat::Jpeg,
    RouterOutputFormat::Webp => OutputFormat::Webp,
  }
}

pub(crate) fn map_fidelity(fidelity: RouterInputFidelity) -> InputFidelity {
  match fidelity {
    RouterInputFidelity::Low => InputFidelity::Low,
    RouterInputFidelity::High => InputFidelity::High,
  }
}

pub(crate) fn input_sources(plan: &ImageGenerationPlan) -> Vec<InputImageSource> {
  plan.image_urls.iter().map(InputImageSource::url).collect()
}

pub(crate) fn mask_source(plan: &ImageGenerationPlan) -> Option<InputImageSource> {
  plan.mask_url.as_ref().map(InputImageSource::url)
}
