/// Omni's `generation_config.video_config.task` — what kind of video work the
/// interaction performs.
///
/// Docs: <https://ai.google.dev/gemini-api/docs/omni>
#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub enum OmniVideoTask {
  /// Generate from the prompt alone.
  TextToVideo,
  /// Animate a provided image.
  ImageToVideo,
  /// Generate guided by reference images.
  ReferenceToVideo,
  /// Edit a previously generated (or uploaded) video.
  Edit,
}

impl OmniVideoTask {
  /// Wire representation.
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::TextToVideo => "text_to_video",
      Self::ImageToVideo => "image_to_video",
      Self::ReferenceToVideo => "reference_to_video",
      Self::Edit => "edit",
    }
  }
}
