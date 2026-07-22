use serde::Serialize;

/// One input image for edit / reference generation.
///
/// The Gemini API does **not** fetch arbitrary public HTTP URLs, so there is no
/// URL variant. Preferred: upload once through the free Files API
/// ([`crate::api::requests::files::upload_image_file`]) and pass the returned
/// [`FileUri`](Self::FileUri) — files persist 48 h and cost nothing. The
/// [`InlineBytes`](Self::InlineBytes) fallback base64-encodes the bytes into the
/// request body (grossly inefficient encoding; use it only for small one-shot
/// inputs where an upload round-trip isn't worth it).
#[derive(Clone, Debug, Serialize)]
pub enum ImageInput {
  /// A Files-API reference (`https://generativelanguage.googleapis.com/v1beta/files/…`).
  FileUri {
    /// The `file.uri` returned by the upload.
    uri: String,
    /// MIME type of the referenced image, e.g. `"image/png"`.
    mime: String,
  },
  /// Raw bytes, base64-encoded onto the wire as an `inline_data` part.
  InlineBytes {
    /// MIME type of the bytes, e.g. `"image/png"`.
    mime: String,
    /// Raw (un-encoded) image bytes.
    bytes: Vec<u8>,
  },
}
