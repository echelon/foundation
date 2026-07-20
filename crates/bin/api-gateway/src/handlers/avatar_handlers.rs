//! Avatar handlers (accounts.md §2): upload/replace with an optional crop, remove the custom
//! avatar, and dismiss/restore the gravatar.
//!
//! The gateway is NOT a CDN — no handler here serves image bytes. Every response carries the
//! avatar's [`crate::avatar_links::AvatarInfo`] (display source + CDN links derived from stored
//! bucket-path pieces), and `/v1/auth/me` reports the same shape, so clients always render from
//! the CDN.
//!
//! Uploads store TWO immutable `user_avatar_files` rows: the original as received, and a baked
//! 512×512 display crop linked through `original_user_avatar_file_id`; the display pointer
//! (`user_avatars.custom_file_id`) references the baked row.

use std::io::Cursor;

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::Json;
use image::imageops::FilterType;
use image::ImageFormat;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use bucket_client::UploadBytesArgs;
use bucket_paths::{mint_bucket_hash, object_key, BucketFolder, ObjectKeyParts};
use db_queries::{user_avatar_files, user_avatars};

use crate::avatar_links::avatar_info;
use crate::error::{ApiError, ApiResult};
use crate::middleware::AuthContext;
use crate::state::AppState;

/// Display size of the baked avatar crop.
const AVATAR_SIZE: u32 = 512;

/* ── POST /v1/auth/avatar — upload/replace, with optional crop ── */

/// Optional crop rect in SOURCE-image pixels. A square viewport's crop rect encodes zoom +
/// translate: zooming in shrinks `crop_size`, panning moves `crop_x`/`crop_y`. All three come
/// together or not at all; without them we center-crop the largest square.
#[derive(Deserialize)]
pub struct CropParams {
  pub crop_x: Option<u32>,
  pub crop_y: Option<u32>,
  pub crop_size: Option<u32>,
}

/// `POST /v1/auth/avatar` — body is the raw image bytes (`fetch(url, { body: file })`).
/// Decodes, optionally crops, bakes a 512×512 PNG, uploads original + baked to R2, records both
/// `user_avatar_files` rows, and points the display at the baked crop. Returns the new avatar
/// (source + CDN links).
pub async fn upload_my_avatar(
  State(state): State<AppState>,
  auth: AuthContext,
  Query(crop): Query<CropParams>,
  body: Bytes,
) -> ApiResult<Json<Value>> {
  if body.is_empty() {
    return Err(ApiError::BadRequest("empty upload".into()));
  }

  // Decode + identify the original format.
  let format = image::guess_format(&body).map_err(|_| ApiError::BadRequest("unrecognized image format".into()))?;
  let (orig_ext, orig_mime) = match format {
    ImageFormat::Png => (".png", "image/png"),
    ImageFormat::Jpeg => (".jpg", "image/jpeg"),
    ImageFormat::WebP => (".webp", "image/webp"),
    ImageFormat::Gif => (".gif", "image/gif"),
    other => return Err(ApiError::BadRequest(format!("unsupported image format: {other:?}"))),
  };
  let img = image::load_from_memory(&body).map_err(|e| ApiError::BadRequest(format!("could not decode image: {e}")))?;
  let (w, h) = (img.width(), img.height());

  // Resolve the crop rect (explicit, or the centered largest square).
  let (cx, cy, csize) = match (crop.crop_x, crop.crop_y, crop.crop_size) {
    (Some(x), Some(y), Some(s)) => {
      if s == 0 || x.saturating_add(s) > w || y.saturating_add(s) > h {
        return Err(ApiError::BadRequest(format!(
          "crop rect ({x},{y}) size {s} is outside the {w}x{h} image"
        )));
      }
      (x, y, s)
    }
    (None, None, None) => {
      let s = w.min(h);
      ((w - s) / 2, (h - s) / 2, s)
    }
    _ => return Err(ApiError::BadRequest("crop_x, crop_y, and crop_size must be supplied together".into())),
  };

  // Inputs are valid — storage is the last gate, so bad requests report 400 even when R2 is off.
  let bucket = state
    .bucket
    .as_ref()
    .ok_or_else(|| ApiError::Internal("avatar storage is not configured on this server".into()))?;

  // Bake the display crop: crop → 512×512 → PNG.
  let baked = img.crop_imm(cx, cy, csize, csize).resize_exact(AVATAR_SIZE, AVATAR_SIZE, FilterType::Lanczos3);
  let mut baked_png: Vec<u8> = Vec::new();
  baked
    .write_to(&mut Cursor::new(&mut baked_png), ImageFormat::Png)
    .map_err(|e| ApiError::Internal(format!("could not encode avatar: {e}")))?;

  // Upload both objects (outside the tx; orphans on late failure are harmless — unguessable keys).
  let orig_hash = mint_bucket_hash();
  let baked_hash = mint_bucket_hash();
  let orig_key = object_key(ObjectKeyParts {
    folder: BucketFolder::Avatars,
    hash: &orig_hash,
    prefix: None,
    suffix: None,
    version: None,
    extension: Some(orig_ext),
  });
  let baked_key = object_key(ObjectKeyParts {
    folder: BucketFolder::Avatars,
    hash: &baked_hash,
    prefix: None,
    suffix: Some("_512"),
    version: None,
    extension: Some(".png"),
  });
  bucket
    .upload_bytes(UploadBytesArgs { object_key: &orig_key, bytes: &body, content_type: Some(orig_mime) })
    .await
    .map_err(|e| ApiError::Internal(format!("avatar upload failed: {e}")))?;
  bucket
    .upload_bytes(UploadBytesArgs { object_key: &baked_key, bytes: &baked_png, content_type: Some("image/png") })
    .await
    .map_err(|e| ApiError::Internal(format!("avatar upload failed: {e}")))?;

  // Record both rows + point the display at the baked crop, atomically.
  let mut tx = state.pool.begin().await?;
  let original = user_avatar_files::create_user_avatar_file(
    &mut *tx,
    user_avatar_files::NewUserAvatarFile {
      user_id: &auth.user.id,
      original_user_avatar_file_id: None,
      avatar_source: "custom_upload",
      bucket_hash: &orig_hash,
      content_hash: &hex::encode(Sha256::digest(&body)),
      bucket_suffix: None,
      bucket_extension: orig_ext,
      width: Some(w as i32),
      height: Some(h as i32),
      bytes: body.len() as i64,
    },
  )
  .await?;
  let baked_row = user_avatar_files::create_user_avatar_file(
    &mut *tx,
    user_avatar_files::NewUserAvatarFile {
      user_id: &auth.user.id,
      original_user_avatar_file_id: Some(&original.id),
      avatar_source: "custom_upload",
      bucket_hash: &baked_hash,
      content_hash: &hex::encode(Sha256::digest(&baked_png)),
      bucket_suffix: Some("_512"),
      bucket_extension: ".png",
      width: Some(AVATAR_SIZE as i32),
      height: Some(AVATAR_SIZE as i32),
      bytes: baked_png.len() as i64,
    },
  )
  .await?;
  user_avatars::set_custom_avatar(&mut *tx, &auth.user.id, &baked_row.id).await?;
  tx.commit().await?;

  respond(&state, &auth).await
}

/* ── DELETE /v1/auth/avatar + gravatar dismiss/restore ── */

/// `DELETE /v1/auth/avatar` — remove the custom avatar; display falls back to the mirrored,
/// undismissed gravatar if there is one, else the monogram.
pub async fn remove_my_avatar(State(state): State<AppState>, auth: AuthContext) -> ApiResult<Json<Value>> {
  user_avatars::remove_custom_avatar(&state.pool, &auth.user.id).await?;
  respond(&state, &auth).await
}

/// `POST /v1/auth/avatar/gravatar/dismiss` — hide the gravatar (reversible; mirror kept).
pub async fn dismiss_my_gravatar(State(state): State<AppState>, auth: AuthContext) -> ApiResult<Json<Value>> {
  user_avatars::set_gravatar_dismissed(&state.pool, &auth.user.id, true).await?;
  respond(&state, &auth).await
}

/// `POST /v1/auth/avatar/gravatar/restore` — un-dismiss the gravatar.
pub async fn restore_my_gravatar(State(state): State<AppState>, auth: AuthContext) -> ApiResult<Json<Value>> {
  user_avatars::set_gravatar_dismissed(&state.pool, &auth.user.id, false).await?;
  respond(&state, &auth).await
}

/// Every mutation answers with the resulting avatar (source + CDN links).
async fn respond(state: &AppState, auth: &AuthContext) -> ApiResult<Json<Value>> {
  let avatar = avatar_info(state, &auth.user.id).await?;
  Ok(Json(json!({ "ok": true, "avatar": avatar })))
}
