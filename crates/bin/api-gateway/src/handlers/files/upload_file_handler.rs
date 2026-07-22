//! `POST /v1/files` — upload an input image (files.md): bytes land in R2 under `uploads/`,
//! a `files` row records the path pieces, and the response returns the `file_…` id (for
//! `image_inputs` / `mask`) plus the derived CDN URL. The gateway stores and points — it
//! never re-serves the bytes.

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::Json;
use image::ImageFormat;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use utoipa::{IntoParams, ToSchema};

use bucket_client::UploadBytesArgs;
use bucket_paths::{cdn_url, mint_bucket_hash, object_key, BucketFolder, ObjectKeyParts};
use db_queries::files;
use ids::{AccountId, FileId};

use crate::error::{ApiError, ApiResult};
use crate::middleware::AuthContext;
use crate::state::AppState;

/// 30 MiB — comfortably above the providers' input limits.
const MAX_UPLOAD_BYTES: usize = 30 * 1024 * 1024;

#[derive(Debug, Deserialize, IntoParams)]
pub struct UploadFileParams {
  /// The account that owns the upload.
  pub account_id: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UploadFileResponse {
  /// Pass this in `image_inputs` / `mask` on the generation endpoints.
  #[schema(value_type = String, example = "file_01k0…")]
  pub file_id: FileId,
  /// Public CDN URL of the stored object.
  pub cdn_url: String,
  pub mime: String,
  pub bytes: i64,
  pub width: u32,
  pub height: u32,
}

/// Raw image bytes in the body (png/jpeg/webp/gif) — `fetch(url, { body: file })`.
#[utoipa::path(
  post, path = "/v1/files",
  tag = "files",
  security(("session_cookie" = [])),
  params(UploadFileParams),
  request_body(content = Vec<u8>, content_type = "application/octet-stream",
    description = "Raw image bytes (png/jpeg/webp/gif)"),
  responses(
    (status = 200, description = "Stored; use `file_id` as a generation input", body = UploadFileResponse),
    (status = 400, description = "Empty/oversized/undecodable upload or unsupported format"),
    (status = 404, description = "Unknown account, or the caller is not a member")
  )
)]
pub async fn upload_file_handler(
  State(state): State<AppState>,
  auth: AuthContext,
  Query(params): Query<UploadFileParams>,
  body: Bytes,
) -> ApiResult<Json<UploadFileResponse>> {
  let account_id: AccountId =
    params.account_id.parse().map_err(|_| ApiError::BadRequest("invalid account id".into()))?;
  db_queries::members::find_membership(&state.pool, &account_id, &auth.user.id)
    .await?
    .ok_or_else(|| ApiError::NotFound("account not found".into()))?;

  if body.is_empty() {
    return Err(ApiError::BadRequest("empty upload".into()));
  }
  if body.len() > MAX_UPLOAD_BYTES {
    return Err(ApiError::BadRequest(format!("upload exceeds {} MiB", MAX_UPLOAD_BYTES / (1024 * 1024))));
  }

  let format = image::guess_format(&body).map_err(|_| ApiError::BadRequest("unrecognized image format".into()))?;
  let (extension, mime) = match format {
    ImageFormat::Png => (".png", "image/png"),
    ImageFormat::Jpeg => (".jpg", "image/jpeg"),
    ImageFormat::WebP => (".webp", "image/webp"),
    ImageFormat::Gif => (".gif", "image/gif"),
    other => return Err(ApiError::BadRequest(format!("unsupported image format: {other:?}"))),
  };
  // Full decode both validates the bytes we're about to publish and yields dimensions.
  let img =
    image::load_from_memory(&body).map_err(|e| ApiError::BadRequest(format!("could not decode image: {e}")))?;

  let bucket = state
    .bucket
    .as_ref()
    .ok_or_else(|| ApiError::Internal("upload storage is not configured on this server".into()))?;
  let hash = mint_bucket_hash();
  let key = object_key(ObjectKeyParts {
    folder: BucketFolder::Uploads,
    hash: &hash,
    prefix: None,
    suffix: None,
    version: None,
    extension: Some(extension),
  });
  bucket
    .upload_bytes(UploadBytesArgs { object_key: &key, bytes: &body, content_type: Some(mime) })
    .await
    .map_err(|e| ApiError::Internal(format!("upload failed: {e}")))?;

  let file = files::create_file(
    &state.pool,
    files::NewFile {
      account_id: &account_id,
      purpose: "input",
      origin: "upload",
      bucket_folder: BucketFolder::Uploads.as_str(),
      bucket_hash: &hash,
      bucket_extension: Some(extension),
      mime,
      bytes: body.len() as i64,
      checksum_sha256: &hex::encode(Sha256::digest(&body)),
    },
  )
  .await?;

  Ok(Json(UploadFileResponse {
    file_id: file.id,
    cdn_url: cdn_url(&state.cdn_base_url, &key),
    mime: mime.to_string(),
    bytes: body.len() as i64,
    width: img.width(),
    height: img.height(),
  }))
}
