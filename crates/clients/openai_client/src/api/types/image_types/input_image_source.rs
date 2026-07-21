/// An input image for the edit modality. The Responses API accepts URLs, base64 `data:`
/// URLs, and file ids — we deliberately support ONLY the efficient two:
///
///   - **`Url`** (strongly preferred): a publicly accessible URL, ultimately managed by us
///     (our CDN). No 33%-bloated base64 payloads shipped per request.
///   - **`FileId`**: a file previously uploaded via the Files API binding
///     (`upload_image_file`) — for sources that can't be public.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputImageSource {
  /// A publicly fetchable URL (https).
  Url(String),
  /// A previously uploaded OpenAI file id (`file-…`) — see `upload_image_file`.
  FileId(String),
}

impl InputImageSource {
  pub fn url(url: impl Into<String>) -> Self {
    Self::Url(url.into())
  }
  pub fn file_id(id: impl Into<String>) -> Self {
    Self::FileId(id.into())
  }

  /// The wire form: `(image_url, file_id)` — exactly one is `Some`.
  pub(crate) fn to_wire(&self) -> (Option<String>, Option<String>) {
    match self {
      Self::Url(url) => (Some(url.clone()), None),
      Self::FileId(id) => (None, Some(id.clone())),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn url_and_file_id_pass_through() {
    assert_eq!(
      InputImageSource::url("https://x.test/a.png").to_wire(),
      (Some("https://x.test/a.png".into()), None)
    );
    assert_eq!(InputImageSource::file_id("file-abc").to_wire(), (None, Some("file-abc".into())));
  }
}
