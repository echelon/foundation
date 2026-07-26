/// A custom-element reference in a video request's `contents`.
///
/// `element_id` comes from the Element Management API; `id` is the
/// caller-chosen index name the prompt refers to as `@id` (e.g. id `"Zhang"`
/// → prompt "…then @Zhang waves…"). `id` must be unique within the task, and
/// names that collide as substrings should be avoided.
#[derive(Clone, Debug)]
pub struct ElementRef {
  pub element_id: String,
  pub id: String,
}
