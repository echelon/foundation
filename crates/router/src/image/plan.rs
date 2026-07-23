//! The provider-agnostic image request — the Any-API superset shape. Model-specific build
//! fns prune what their model can't do (recording adjustments) and map the rest.

use serde::{Deserialize, Serialize};

/// Which catalog image model to route. Hard-coded set (catalog-as-code).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouterImageModel {
  OpenaiGptImageV1,
  OpenaiGptImageV1Mini,
  OpenaiGptImageV1_5,
  OpenaiGptImageV2,
  GoogleNanoBanana,
  GoogleNanoBananaPro,
  GoogleNanoBanana2,
  GoogleNanoBanana2Lite,
}

impl RouterImageModel {
  /// Route from the canonical catalog slug.
  pub fn from_slug(slug: &str) -> Option<Self> {
    match slug {
      "openai_gpt_image_1" => Some(Self::OpenaiGptImageV1),
      "openai_gpt_image_1_mini" => Some(Self::OpenaiGptImageV1Mini),
      "openai_gpt_image_1_5" => Some(Self::OpenaiGptImageV1_5),
      "openai_gpt_image_2" => Some(Self::OpenaiGptImageV2),
      "google_nano_banana" => Some(Self::GoogleNanoBanana),
      "google_nano_banana_pro" => Some(Self::GoogleNanoBananaPro),
      "google_nano_banana_2" => Some(Self::GoogleNanoBanana2),
      "google_nano_banana_2_lite" => Some(Self::GoogleNanoBanana2Lite),
      _ => None,
    }
  }

  pub fn slug(&self) -> &'static str {
    match self {
      Self::OpenaiGptImageV1 => "openai_gpt_image_1",
      Self::OpenaiGptImageV1Mini => "openai_gpt_image_1_mini",
      Self::OpenaiGptImageV1_5 => "openai_gpt_image_1_5",
      Self::OpenaiGptImageV2 => "openai_gpt_image_2",
      Self::GoogleNanoBanana => "google_nano_banana",
      Self::GoogleNanoBananaPro => "google_nano_banana_pro",
      Self::GoogleNanoBanana2 => "google_nano_banana_2",
      Self::GoogleNanoBanana2Lite => "google_nano_banana_2_lite",
    }
  }

  pub fn spec(&self) -> &'static catalog::ModelSpec {
    catalog::by_slug(self.slug()).expect("router models are in the catalog")
  }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouterImageSize {
  Square1024,
  Landscape1536x1024,
  Portrait1024x1536,
  Auto,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouterImageQuality {
  Low,
  Medium,
  High,
  Auto,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouterOutputFormat {
  Png,
  Jpeg,
  Webp,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouterInputFidelity {
  Low,
  High,
}

/// The plan: everything ANY image model could take. Build fns prune per model.
#[derive(Clone, Debug)]
pub struct ImageGenerationPlan {
  pub model: RouterImageModel,
  pub prompt: String,
  /// Edit inputs — OUR public CDN urls (inputs are mirrored/uploaded before planning).
  pub image_urls: Vec<String>,
  pub mask_url: Option<String>,
  /// How many images (1–10). Fans out to N provider calls (the Responses image tool has
  /// no `n` — see the openai-multi-image-fanout decision).
  pub n: u8,
  pub size: Option<RouterImageSize>,
  pub quality: Option<RouterImageQuality>,
  pub output_format: Option<RouterOutputFormat>,
  /// Edit fidelity — pruned (with a recorded adjustment) for models that lack it.
  pub input_fidelity: Option<RouterInputFidelity>,
}

impl ImageGenerationPlan {
  pub fn new(model: RouterImageModel, prompt: impl Into<String>) -> Self {
    Self {
      model,
      prompt: prompt.into(),
      image_urls: Vec::new(),
      mask_url: None,
      n: 1,
      size: None,
      quality: None,
      output_format: None,
      input_fidelity: None,
    }
  }
}
