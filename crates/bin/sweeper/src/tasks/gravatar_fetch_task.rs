//! Gravatar probe + mirror (accounts.md §2), ported from the retired `gravatar-job`
//! binary. Scans `user_avatars` rows needing a probe (`gravatar_checked_at IS NULL` — set
//! at signup, reset on email change), fetches from gravatar.com, and mirrors hits into R2
//! as `user_avatar_files` rows so customer consoles never call gravatar.com.
//!
//! Fetch: `GET https://gravatar.com/avatar/{sha256(trim+lower(email))}?s=512&d=404`.
//! `d=404` makes gravatar answer 404 when the address has no avatar (200 ⇒ bytes in hand).

use std::{io::Cursor, time::Duration};

use anyhow::{bail, ensure};
use bucket_client::{BucketClient, UploadBytesArgs};
use bucket_paths::{mint_bucket_hash, object_key, BucketFolder, ObjectKeyParts};
use db_queries::{user_avatar_files, user_avatars, UserAvatar};
use image::{ImageFormat, ImageReader};
use reqwest::StatusCode;
use sha2::{Digest, Sha256};
use sqlx::PgConnection;

use super::{SweeperTask, TaskContext};

/// How many avatars to probe per pass.
const BATCH_SIZE: i64 = 25;
const MAX_IMAGE_BYTES: usize = 4 * 1024 * 1024;
const MAX_DIMENSION: u32 = 1024;

pub struct GravatarFetchTask;

impl SweeperTask for GravatarFetchTask {
  fn name(&self) -> &'static str {
    "gravatar_fetch"
  }

  /// Probe one batch of unpolled avatars. A single row's failure is isolated: on a
  /// fetch/upload error the row stays unpolled (retried next pass) and the loop moves on
  /// — the pass only errors on a DB-level failure fetching the work itself.
  async fn run_once(&self, ctx: &TaskContext) -> anyhow::Result<usize> {
    let Some(bucket) = &ctx.bucket else {
      return Ok(0);
    };
    let batch = user_avatars::list_unpolled_avatars(&ctx.pool, BATCH_SIZE).await?;

    let mut processed = 0;
    for avatar in batch {
      let result = match avatar.gravatar_hash.as_deref() {
        Some(hash) => match fetch_gravatar(&ctx.http, hash).await {
          Ok(Some(image)) => mirror_gravatar(&ctx.pool, bucket, &avatar, image).await,
          Ok(None) => record_missing(&ctx.pool, &avatar).await,
          Err(error) => Err(error),
        },
        None => record_missing(&ctx.pool, &avatar).await,
      };
      match result {
        Ok(true) => processed += 1,
        Ok(false) => {}
        Err(error) => tracing::warn!(%error, user_id = %avatar.user_id, "Gravatar import failed; will retry"),
      }
    }
    Ok(processed)
  }
}

struct GravatarImage {
  bytes: Vec<u8>,
  extension: &'static str,
  content_type: &'static str,
  width: i32,
  height: i32,
}

/// A 404 means the email has no image. Other failures should leave the current mirror intact.
async fn fetch_gravatar(http: &reqwest::Client, hash: &str) -> anyhow::Result<Option<GravatarImage>> {
  ensure!(hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()), "invalid Gravatar hash");
  let url = format!("https://gravatar.com/avatar/{hash}?s=512&d=404");
  let mut response = http.get(url).timeout(Duration::from_secs(10)).send().await
    .map_err(reqwest::Error::without_url)?;
  let status = response.status();
  if status == StatusCode::NOT_FOUND {
    return Ok(None);
  }
  ensure!(status == StatusCode::OK, "Gravatar returned {status}");
  let mut bytes = Vec::new();
  while let Some(chunk) = response.chunk().await.map_err(reqwest::Error::without_url)? {
    ensure!(chunk.len() <= MAX_IMAGE_BYTES - bytes.len(), "Gravatar image is too large");
    bytes.extend_from_slice(&chunk);
  }
  let mut reader = ImageReader::new(Cursor::new(&bytes)).with_guessed_format()?;
  let (extension, content_type) = match reader.format() {
    Some(ImageFormat::Png) => (".png", "image/png"),
    Some(ImageFormat::Jpeg) => (".jpg", "image/jpeg"),
    Some(ImageFormat::WebP) => (".webp", "image/webp"),
    Some(ImageFormat::Gif) => (".gif", "image/gif"),
    _ => bail!("unsupported Gravatar image format"),
  };
  let mut limits = image::Limits::default();
  limits.max_image_width = Some(MAX_DIMENSION);
  limits.max_image_height = Some(MAX_DIMENSION);
  limits.max_alloc = Some(16 * 1024 * 1024);
  reader.limits(limits);
  let decoded = reader.decode()?;
  Ok(Some(GravatarImage {
    bytes,
    extension,
    content_type,
    width: decoded.width() as i32,
    height: decoded.height() as i32,
  }))
}

/// A remove, refresh or email change can supersede this probe while its HTTP request is in flight.
async fn lock_pending_probe(connection: &mut PgConnection, avatar: &UserAvatar) -> sqlx::Result<bool> {
  Ok(sqlx::query_scalar::<_, i32>(
    "SELECT 1 FROM user_avatars WHERE user_id = $1 AND gravatar_checked_at IS NULL \
     AND updated_at = $2 FOR UPDATE",
  )
  .bind(avatar.user_id.as_str())
  .bind(avatar.updated_at)
  .fetch_optional(connection)
  .await?
  .is_some())
}

async fn record_missing(pool: &db::PgPool, avatar: &UserAvatar) -> anyhow::Result<bool> {
  let mut tx = pool.begin().await?;
  if !lock_pending_probe(&mut tx, avatar).await? {
    return Ok(false);
  }
  user_avatars::mark_gravatar_checked(&mut *tx, &avatar.user_id, false).await?;
  tx.commit().await?;
  Ok(true)
}

/// Mirror gravatar bytes into R2 + record the `user_avatar_files` row + promote the avatar
/// (accounts.md §2). Upload happens OUTSIDE the db transaction; an orphaned object on a late
/// failure is harmless (unguessable key, retried row).
async fn mirror_gravatar(
  pool: &db::PgPool,
  bucket: &BucketClient,
  avatar: &UserAvatar,
  image: GravatarImage,
) -> anyhow::Result<bool> {
  let bucket_hash = mint_bucket_hash();
  let key = object_key(ObjectKeyParts {
    folder: BucketFolder::Avatars,
    hash: &bucket_hash,
    prefix: None,
    suffix: None,
    version: None,
    extension: Some(image.extension),
  });

  bucket
    .upload_bytes(UploadBytesArgs { object_key: &key, bytes: &image.bytes, content_type: Some(image.content_type) })
    .await
    .map_err(|e| anyhow::anyhow!("upload {key}: {e}"))?;

  let content_hash = hex::encode(Sha256::digest(&image.bytes));

  let mut tx = pool.begin().await?;
  if !lock_pending_probe(&mut tx, avatar).await? {
    return Ok(false);
  }
  let file = user_avatar_files::create_user_avatar_file(
    &mut *tx,
    user_avatar_files::NewUserAvatarFile {
      user_id: &avatar.user_id,
      original_user_avatar_file_id: None,
      avatar_source: enums::AvatarFileSource::GravatarMirror,
      bucket_hash: &bucket_hash,
      content_hash: &content_hash,
      bucket_suffix: None,
      bucket_extension: image.extension,
      width: Some(image.width),
      height: Some(image.height),
      bytes: image.bytes.len() as i64,
    },
  )
  .await?;
  user_avatars::set_gravatar_mirrored(&mut *tx, &avatar.user_id, &file.id).await?;
  tx.commit().await?;
  Ok(true)
}
