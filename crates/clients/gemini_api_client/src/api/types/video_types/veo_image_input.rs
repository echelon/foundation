use serde::Serialize;

/// The conditioning image for image-to-video generation.
///
/// As with image inputs, the Gemini API does not fetch arbitrary public HTTP
/// URLs. Prefer a Files-API reference ([`FileUri`](Self::FileUri), via
/// [`crate::api::requests::files::upload_image_file`]); [`InlineBytes`](Self::InlineBytes)
/// base64-encodes the bytes into the request.
#[derive(Clone, Debug, Serialize)]
pub enum VeoImageInput {
  /// A Files-API reference.
  FileUri {
    /// The `file.uri` returned by the upload.
    uri: String,
    /// MIME type of the referenced image.
    mime: String,
  },
  /// Raw bytes, base64-encoded onto the wire as `bytesBase64Encoded`.
  InlineBytes {
    /// MIME type of the bytes, e.g. `"image/png"`.
    mime: String,
    /// Raw (un-encoded) image bytes.
    bytes: Vec<u8>,
  },
}
