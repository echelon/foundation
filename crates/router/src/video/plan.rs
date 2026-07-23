//! The provider-agnostic VIDEO request — the Any-API superset shape. Build fns
//! prune/coerce per model and record adjustments (mirrors `image::plan`).

use serde::{Deserialize, Serialize};

/// Which catalog video model to route. Hard-coded set (catalog-as-code).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouterVideoModel {
  GoogleVeo3_1,
  GoogleVeo3_1Fast,
  GoogleVeo3_1Lite,
}

impl RouterVideoModel {
  pub fn from_slug(slug: &str) -> Option<Self> {
    match slug {
      "google_veo_3_1" => Some(Self::GoogleVeo3_1),
      "google_veo_3_1_fast" => Some(Self::GoogleVeo3_1Fast),
      "google_veo_3_1_lite" => Some(Self::GoogleVeo3_1Lite),
      _ => None,
    }
  }

  pub fn slug(&self) -> &'static str {
    match self {
      Self::GoogleVeo3_1 => "google_veo_3_1",
      Self::GoogleVeo3_1Fast => "google_veo_3_1_fast",
      Self::GoogleVeo3_1Lite => "google_veo_3_1_lite",
    }
  }

  pub fn spec(&self) -> &'static catalog::ModelSpec {
    catalog::by_slug(self.slug()).expect("router video models are in the catalog")
  }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouterVideoResolution {
  P720,
  P1080,
  K4,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouterVideoDuration {
  S4,
  S6,
  S8,
}

impl RouterVideoDuration {
  pub fn seconds(&self) -> i64 {
    match self {
      Self::S4 => 4,
      Self::S6 => 6,
      Self::S8 => 8,
    }
  }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouterVideoAspectRatio {
  Landscape16x9,
  Portrait9x16,
}

/// The plan: everything ANY video model could take.
#[derive(Clone, Debug)]
pub struct VideoGenerationPlan {
  pub model: RouterVideoModel,
  pub prompt: String,
  pub negative_prompt: Option<String>,
  /// Optional image-to-video conditioning frame — OUR public CDN url.
  pub image_url: Option<String>,
  /// How many videos (1–10). One provider operation per video — fans out to
  /// N slices when n > 1 (Veo has no batch parameter).
  pub n: u8,
  pub resolution: Option<RouterVideoResolution>,
  pub duration: Option<RouterVideoDuration>,
  pub aspect_ratio: Option<RouterVideoAspectRatio>,
}
