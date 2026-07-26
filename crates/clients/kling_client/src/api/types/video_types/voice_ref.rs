/// A custom-voice reference in a Kling 2.6 image-to-video request's
/// `contents` (voice control).
///
/// `voice_id` comes from the Voice Management API (or the preset library);
/// `id` is the caller-chosen index name the prompt refers to as `@id`
/// (e.g. id `"1"` → prompt "…@1 whispers softly…"). Requires
/// `audio: native`; at most 2 voices per task.
#[derive(Clone, Debug)]
pub struct VoiceRef {
  pub voice_id: String,
  pub id: String,
}
