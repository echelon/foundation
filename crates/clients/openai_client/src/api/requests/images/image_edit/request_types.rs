/// One input image for an edit (or reference composite). `/v1/images/edits` accepts up to
/// 16 for GPT Image models, each supplied as a multipart file part.
#[derive(Clone)]
pub struct InputImage {
  pub bytes: Vec<u8>,
  /// Filename hint sent in the multipart part (extension drives the server's MIME sniff).
  pub filename: String,
  /// MIME of the bytes, e.g. `image/png`.
  pub mime: String,
}

impl InputImage {
  pub fn png(bytes: Vec<u8>, filename: impl Into<String>) -> Self {
    Self { bytes, filename: filename.into(), mime: "image/png".into() }
  }
}

// Debug hides the raw bytes so a `{:?}` on an enclosing request doesn't dump megabytes.
impl std::fmt::Debug for InputImage {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("InputImage")
      .field("filename", &self.filename)
      .field("mime", &self.mime)
      .field("bytes", &format_args!("<{} bytes>", self.bytes.len()))
      .finish()
  }
}
