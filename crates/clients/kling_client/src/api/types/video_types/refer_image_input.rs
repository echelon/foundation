/// A reference image in an omni-video request's `contents`.
///
/// `url` is a hosted URL or raw Base64. `id` is an optional caller-chosen
/// index name the prompt refers to as `@id` (e.g. `"image_1"` →
/// "…@image_1's jacket…").
#[derive(Clone, Debug)]
pub struct ReferImageInput {
  pub url: String,
  pub id: Option<String>,
}
