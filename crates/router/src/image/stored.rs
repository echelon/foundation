//! The sanitized, RESOLVED image request persisted on `jobs.request_params` — everything
//! a dispatcher (gateway or worker binary) needs to rebuild the router plan (file ids
//! already resolved to CDN urls). Lives in the router so every dispatcher shares ONE
//! stored shape.

use serde::{Deserialize, Serialize};

use crate::image::plan::{
  ImageGenerationPlan, RouterImageModel, RouterImageQuality, RouterImageSize, RouterInputFidelity,
  RouterOutputFormat,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredImageParams {
  pub model: String,
  pub prompt: String,
  #[serde(default)]
  pub image_urls: Vec<String>,
  #[serde(default)]
  pub mask_url: Option<String>,
  pub n: u8,
  #[serde(default)]
  pub size: Option<RouterImageSize>,
  #[serde(default)]
  pub quality: Option<RouterImageQuality>,
  #[serde(default)]
  pub output_format: Option<RouterOutputFormat>,
  #[serde(default)]
  pub input_fidelity: Option<RouterInputFidelity>,
}

impl StoredImageParams {
  pub fn to_plan(&self) -> Option<ImageGenerationPlan> {
    let model = RouterImageModel::from_slug(&self.model)?;
    Some(ImageGenerationPlan {
      model,
      prompt: self.prompt.clone(),
      image_urls: self.image_urls.clone(),
      mask_url: self.mask_url.clone(),
      n: self.n,
      size: self.size,
      quality: self.quality,
      output_format: self.output_format,
      input_fidelity: self.input_fidelity,
    })
  }
}
