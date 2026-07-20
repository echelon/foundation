//! Avatar handlers (accounts.md §2): serve the current avatar, upload/replace with an optional
//! crop, remove the custom avatar, and dismiss/restore the gravatar.
//!
//! Bytes live in R2 (`user_avatar_files` rows hold the bucket-path pieces; keys derived via
//! `bucket_paths`, files.md §2). Uploads store TWO immutable rows: the original as received, and
//! a baked 512×512 display crop linked through `original_user_avatar_file_id` — the display
//! pointer (`user_avatars.custom_file_id`) references the baked row.

use std::io::Cursor;

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use image::imageops::FilterType;
use image::ImageFormat;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use bucket_client::UploadBytesArgs;
use bucket_paths::{mint_bucket_hash, object_key, BucketFolder};
use db_queries::{user_avatar_files, user_avatars};
use ids::UserAvatarFileId;

use crate::error::{ApiError, ApiResult};
use crate::middleware::AuthContext;
use crate::state::AppState;

/// Display size of the baked avatar crop.
const AVATAR_SIZE: u32 = 512;

/* ── GET /v1/auth/avatar ── */

/// Serve the caller's avatar image: the custom upload or mirrored gravatar from R2 (with an
/// `ETag`/304 so browsers cache across replacements), falling back to proxying gravatar.com for
/// found-but-not-yet-mirrored gravatars. 404 → the frontend shows the monogram.
pub async fn get_my_avatar(
  State(state): State<AppState>,
  auth: AuthContext,
  headers: HeaderMap,
) -> ApiResult<Response> {
  let avatar = user_avatars::find_user_avatar_by_user(&state.pool, &auth.user.id)
    .await?
    .ok_or_else(|| ApiError::NotFound("no avatar".into()))?;

  // Which file backs the display source (accounts.md §2 precedence)?
  let file_id: Option<UserAvatarFileId> = match avatar.source.as_str() {
    "custom" => avatar.custom_file_id.clone(),
    "gravatar" if !avatar.is_gravatar_dismissed => avatar.gravatar_file_id.clone(),
    _ => None,
  };

  if let (Some(file_id), Some(bucket)) = (file_id, state.bucket.as_ref()) {
    let file = user_avatar_files::find_user_avatar_file_by_id(&state.pool, &file_id)
      .await?
      .ok_or_else(|| ApiError::NotFound("avatar file missing".into()))?;

    // Content-addressed caching: the content hash IS the ETag.
    let etag = format!("\"{}\"", file.content_hash);
    if headers.get("if-none-match").and_then(|v| v.to_str().ok()) == Some(etag.as_str()) {
      return Ok(StatusCode::NOT_MODIFIED.into_response());
    }

    let key = object_key(
      BucketFolder::Avatars,
      &file.bucket_hash,
      None,
      file.bucket_suffix.as_deref(),
      None,
      Some(&file.bucket_extension),
    );
    let (bytes, _) = bucket
      .download_bytes(&key)
      .await
      .map_err(|e| ApiError::Internal(format!("avatar download failed: {e}")))?;

    return Ok(
      (
        StatusCode::OK,
        [
          ("content-type", content_type_for(&file.bucket_extension).to_string()),
          ("etag", etag),
          ("cache-control", "private, no-cache".to_string()),
        ],
        bytes,
      )
        .into_response(),
    );
  }

  // Found-but-unmirrored gravatar (e.g. storage was off when the job ran): proxy it.
  if avatar.source == "gravatar" && !avatar.is_gravatar_dismissed {
    if let Some(hash) = avatar.gravatar_hash.as_deref() {
      return proxy_gravatar(hash).await;
    }
  }
  Err(ApiError::NotFound("no avatar image".into()))
}

async fn proxy_gravatar(hash: &str) -> ApiResult<Response> {
  let url = format!("https://gravatar.com/avatar/{hash}?s=256&d=404");
  let upstream = client_core::http_client("foundation/api-gateway")
    .get(&url)
    .send()
    .await
    .map_err(|e| ApiError::Internal(format!("gravatar fetch failed: {e}")))?;
  if !upstream.status().is_success() {
    return Err(ApiError::NotFound("no avatar image".into()));
  }
  let content_type = upstream
    .headers()
    .get("content-type")
    .and_then(|v| v.to_str().ok())
    .unwrap_or("image/png")
    .to_string();
  let bytes = upstream
    .bytes()
    .await
    .map_err(|e| ApiError::Internal(format!("gravatar read failed: {e}")))?;
  Ok(
    (
      StatusCode::OK,
      [("content-type", content_type), ("cache-control", "private, max-age=3600".to_string())],
      bytes,
    )
      .into_response(),
  )
}

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
/// `user_avatar_files` rows, and points the display at the baked crop.
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
  let orig_key = object_key(BucketFolder::Avatars, &orig_hash, None, None, None, Some(orig_ext));
  let baked_key = object_key(BucketFolder::Avatars, &baked_hash, None, Some("_512"), None, Some(".png"));
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

  Ok(Json(json!({ "ok": true, "avatar_source": "custom" })))
}

/* ── DELETE /v1/auth/avatar + gravatar dismiss/restore ── */

/// `DELETE /v1/auth/avatar` — remove the custom avatar; display falls back to the mirrored,
/// undismissed gravatar if there is one, else the monogram.
pub async fn remove_my_avatar(State(state): State<AppState>, auth: AuthContext) -> ApiResult<Json<Value>> {
  user_avatars::remove_custom_avatar(&state.pool, &auth.user.id).await?;
  respond_with_source(&state, &auth).await
}

/// `POST /v1/auth/avatar/gravatar/dismiss` — hide the gravatar (reversible; mirror kept).
pub async fn dismiss_my_gravatar(State(state): State<AppState>, auth: AuthContext) -> ApiResult<Json<Value>> {
  user_avatars::set_gravatar_dismissed(&state.pool, &auth.user.id, true).await?;
  respond_with_source(&state, &auth).await
}

/// `POST /v1/auth/avatar/gravatar/restore` — un-dismiss the gravatar.
pub async fn restore_my_gravatar(State(state): State<AppState>, auth: AuthContext) -> ApiResult<Json<Value>> {
  user_avatars::set_gravatar_dismissed(&state.pool, &auth.user.id, false).await?;
  respond_with_source(&state, &auth).await
}

async fn respond_with_source(state: &AppState, auth: &AuthContext) -> ApiResult<Json<Value>> {
  let source = user_avatars::find_user_avatar_by_user(&state.pool, &auth.user.id)
    .await?
    .map(|a| a.source)
    .unwrap_or_else(|| "letter".into());
  Ok(Json(json!({ "ok": true, "avatar_source": source })))
}

/// Serve content type for a stored `bucket_extension`.
fn content_type_for(extension: &str) -> &'static str {
  match extension {
    ".png" => "image/png",
    ".jpg" | ".jpeg" => "image/jpeg",
    ".webp" => "image/webp",
    ".gif" => "image/gif",
    _ => "application/octet-stream",
  }
}
