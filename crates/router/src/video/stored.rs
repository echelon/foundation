//! The sanitized, RESOLVED video request persisted on `jobs.request_params` —
//! everything a dispatcher (gateway or worker binary) needs to rebuild the
//! video route. Mirrors `image::stored`.

use serde::{Deserialize, Serialize};

use crate::video::plan::{
  RouterVideoAspectRatio, RouterVideoDuration, RouterVideoModel, RouterVideoResolution, VideoGenerationPlan,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredVideoParams {
  pub model: String,
  pub prompt: String,
  #[serde(default)]
  pub negative_prompt: Option<String>,
  /// Image-to-video conditioning frame — OUR public CDN url.
  #[serde(default)]
  pub image_url: Option<String>,
  pub n: u8,
  #[serde(default)]
  pub resolution: Option<RouterVideoResolution>,
  #[serde(default)]
  pub duration: Option<RouterVideoDuration>,
  #[serde(default)]
  pub aspect_ratio: Option<RouterVideoAspectRatio>,
}

impl StoredVideoParams {
  pub fn to_plan(&self) -> Option<VideoGenerationPlan> {
    let model = RouterVideoModel::from_slug(&self.model)?;
    Some(VideoGenerationPlan {
      model,
      prompt: self.prompt.clone(),
      negative_prompt: self.negative_prompt.clone(),
      image_url: self.image_url.clone(),
      n: self.n,
      resolution: self.resolution,
      duration: self.duration,
      aspect_ratio: self.aspect_ratio,
    })
  }
}
