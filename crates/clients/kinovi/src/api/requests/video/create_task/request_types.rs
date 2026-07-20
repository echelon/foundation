use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};

use crate::api::types::video_types::seedance_model::SeedanceModel;
use crate::api::types::video_types::video_aspect_ratio::VideoAspectRatio;
use crate::api::types::video_types::video_resolution::VideoResolution;

/// The material part of a Kinovi `createTask` request. Derives no key material, so
/// it can be logged/persisted independently of the API key (which lives in
/// [`super::create_task::CreateTaskArgs`]).
///
/// Serializes to Kinovi's wire shape:
///
/// ```json
/// {
///   "model": "seedance-20",
///   "inputs": { "prompt": "...", "duration": "5", "outputResolution": "720p",
///               "aspectRatio": "16:9", "mode": "keyframe", "imageUrls": ["..."] },
///   "callBackUrl": "https://..."
/// }
/// ```
///
/// `mode` is derived: it is emitted as `"keyframe"` (with `imageUrls`) exactly when
/// [`VideoInputs::image_urls`] is non-empty (image-to-video); text-to-video omits both.
#[derive(Clone, Debug)]
pub struct CreateTaskRequest {
  /// Seedance model id.
  pub model: SeedanceModel,

  /// Generation inputs (prompt, duration, resolution, …).
  pub inputs: VideoInputs,

  /// Optional completion webhook. Kinovi POSTs the finished task to this URL.
  pub callback_url: Option<String>,
}

/// The `inputs` object of a `createTask` request.
#[derive(Clone, Debug)]
pub struct VideoInputs {
  /// Text prompt describing the video. Required.
  pub prompt: String,

  /// Clip length in seconds (Kinovi accepts 4..=15). Serialized as a STRING
  /// (`"5"`) to match the wire format.
  pub duration_seconds: u32,

  /// Output resolution tier.
  pub resolution: VideoResolution,

  /// Aspect ratio. Omitted from the wire body when `None` (Kinovi picks a default).
  pub aspect_ratio: Option<VideoAspectRatio>,

  /// First-frame / reference image URLs for image-to-video. When non-empty, the
  /// request is sent with `mode: "keyframe"` and these `imageUrls`. Empty ⇒
  /// text-to-video (neither field is emitted).
  pub image_urls: Vec<String>,
}

impl VideoInputs {
  /// Whether this is an image-to-video request (has at least one keyframe URL).
  pub fn is_image_to_video(&self) -> bool {
    !self.image_urls.is_empty()
  }
}

impl Serialize for CreateTaskRequest {
  fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
    let field_count = if self.callback_url.is_some() { 3 } else { 2 };
    let mut st = s.serialize_struct("CreateTaskRequest", field_count)?;
    st.serialize_field("model", &self.model)?;
    st.serialize_field("inputs", &self.inputs)?;
    if let Some(callback_url) = &self.callback_url {
      st.serialize_field("callBackUrl", callback_url)?;
    }
    st.end()
  }
}

impl Serialize for VideoInputs {
  fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
    let is_i2v = self.is_image_to_video();
    // prompt + duration + outputResolution, plus optional aspectRatio and the
    // (mode + imageUrls) pair for image-to-video.
    let mut field_count = 3;
    if self.aspect_ratio.is_some() {
      field_count += 1;
    }
    if is_i2v {
      field_count += 2;
    }

    let mut st = s.serialize_struct("VideoInputs", field_count)?;
    st.serialize_field("prompt", &self.prompt)?;
    // Duration is a STRING of seconds on the wire.
    st.serialize_field("duration", &self.duration_seconds.to_string())?;
    st.serialize_field("outputResolution", self.resolution.as_str())?;
    if let Some(aspect_ratio) = self.aspect_ratio {
      st.serialize_field("aspectRatio", aspect_ratio.as_str())?;
    }
    if is_i2v {
      st.serialize_field("mode", "keyframe")?;
      st.serialize_field("imageUrls", &self.image_urls)?;
    }
    st.end()
  }
}

// ── Response ──

/// Wire response from `POST /jobs/createTask`.
#[derive(Deserialize, Debug)]
pub(crate) struct CreateTaskResponseBody {
  #[serde(rename = "taskId")]
  pub task_id: String,
}
