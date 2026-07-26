use serde::{Deserialize, Serialize};

/// Video reference for a `video_refer` (video character) element: at most one
/// video, `.mp4/.mov`, 3–8s, 1080P, aspect 16:9 or 9:16, ≤200MB. If the video
/// contains a human voice, Kling automatically creates and binds a custom
/// voice.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ElementVideoList {
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub refer_videos: Vec<ReferVideo>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReferVideo {
  pub video_url: String,
}
