/// A reference video in an omni-video request's `contents` (hosted URL only —
/// no Base64). `.mp4/.mov`, with per-endpoint size/duration/pixel limits.
///
/// `id` is an optional caller-chosen index name the prompt refers to as
/// `@id` (e.g. `"video_1"`).
#[derive(Clone, Debug)]
pub struct ReferVideoInput {
  pub url: String,
  pub id: Option<String>,
}
