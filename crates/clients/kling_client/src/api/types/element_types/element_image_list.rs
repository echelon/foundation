use serde::{Deserialize, Serialize};

/// Image references for an `image_refer` (multi-image) element: exactly one
/// frontal image plus 1–3 additional reference images. Images are URL or raw
/// Base64, `.jpg/.jpeg/.png`, ≤10MB, sides ≥300px, aspect 1:2.5–2.5:1.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ElementImageList {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub frontal_image: Option<String>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub refer_images: Vec<ReferImage>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReferImage {
  pub image_url: String,
}
