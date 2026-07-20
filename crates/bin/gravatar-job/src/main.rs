//! gravatar-job — a standalone background worker that fills in gravatar avatars.
//!
//! It scans `user_avatars` for rows that still need a gravatar probe (`gravatar_checked_at IS
//! NULL` — set on signup and reset on every email change), fetches from `gravatar.com` for
//! each, and — when object storage is configured — **mirrors the image into R2** as a
//! `user_avatar_files` row (`avatar_source = gravatar_mirror`, accounts.md §2), so customers'
//! consoles never call gravatar.com. Without storage config it degrades to existence-probing
//! only. Deliberately simple: ONE main loop, batch of work, graceful failure/backoff.
//!
//! Fetch: `GET https://gravatar.com/avatar/{sha256(trim+lower(email))}?s=512&d=404`.
//! `d=404` makes gravatar answer 404 when the address has no avatar (200 ⇒ bytes in hand).

use std::time::Duration;

use bucket_client::{BucketClient, BucketClientBuilder, UploadBytesArgs};
use bucket_paths::{mint_bucket_hash, object_key, BucketFolder, ObjectKeyParts};
use db_queries::{user_avatar_files, user_avatars};
use sha2::{Digest, Sha256};

/// How many avatars to probe per loop iteration.
const BATCH_SIZE: i64 = 25;
/// Idle wait when there's no work (or after a DB error) before scanning again.
const IDLE_WAIT: Duration = Duration::from_secs(15);
/// Backoff after a probe/transport error spike, to avoid hammering a flaky network.
const ERROR_BACKOFF: Duration = Duration::from_secs(30);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  // Env files first (convention: crates/bin/gravatar-job/config/gravatar-job.{env}.env), so
  // `cargo run -p gravatar_job` needs no hand-supplied env vars and RUST_LOG from a file works.
  let loaded = config::load_env_files("gravatar-job");

  tracing_subscriber::fmt()
    .with_env_filter(
      tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,gravatar_job=debug".into()),
    )
    .init();

  for path in &loaded {
    tracing::info!("loaded env file: {}", path.display());
  }

  let cfg = config::Config::load()?;
  let pool = db::connect(&cfg.database_url).await?;

  let bucket = match &cfg.storage {
    Some(s) => Some(
      BucketClientBuilder::new()
        .endpoint(&s.endpoint)
        .access_key(&s.access_key_id)
        .secret_key(&s.secret_access_key)
        .region_name(&s.region)
        .bucket_name(&s.bucket_name)
        .build()
        .map_err(|e| anyhow::anyhow!("bucket client: {e}"))?,
    ),
    None => {
      tracing::warn!("R2 storage not configured — probing existence only, not mirroring images");
      None
    }
  };
  tracing::info!(mirroring = bucket.is_some(), "gravatar-job connected to database; starting poll loop");

  let http = client_core::http_client("foundation/gravatar-job");

  loop {
    // ctrl-c wins against whatever we're doing, so shutdown is prompt even under a backlog.
    tokio::select! {
      _ = tokio::signal::ctrl_c() => {
        tracing::info!("shutdown signal received — exiting");
        break;
      }
      outcome = run_batch(&pool, &http, bucket.as_ref()) => {
        match outcome {
          // No work — wait before scanning again.
          Ok(0) => if wait_or_shutdown(IDLE_WAIT).await { break; },
          Ok(n) => tracing::info!(processed = n, "processed a batch"), // loop straight back for more
          Err(e) => {
            tracing::error!(error = %e, "batch failed — backing off");
            if wait_or_shutdown(ERROR_BACKOFF).await {
              break;
            }
          }
        }
      }
    }
  }

  Ok(())
}

/// Sleep for `dur`, returning `true` early if a shutdown (ctrl-c) is requested meanwhile.
async fn wait_or_shutdown(dur: Duration) -> bool {
  tokio::select! {
    _ = tokio::time::sleep(dur) => false,
    _ = tokio::signal::ctrl_c() => true,
  }
}

/// Probe one batch of unpolled avatars. Returns how many were processed (0 ⇒ idle). A single row's
/// failure is isolated: on a fetch/upload error we leave the row unpolled (retried next pass) and
/// move on — the batch only errors on a DB-level failure fetching the work.
async fn run_batch(pool: &db::PgPool, http: &reqwest::Client, bucket: Option<&BucketClient>) -> anyhow::Result<usize> {
  let batch = user_avatars::list_unpolled_avatars(pool, BATCH_SIZE).await?;
  if batch.is_empty() {
    return Ok(0);
  }

  let mut processed = 0;
  for avatar in batch {
    let Some(hash) = avatar.gravatar_hash.as_deref() else {
      // No hash to probe with — mark checked (not found) so it leaves the queue.
      let _ = user_avatars::mark_gravatar_checked(pool, &avatar.user_id, false).await;
      processed += 1;
      continue;
    };

    match fetch_gravatar(http, hash).await {
      Ok(None) => {
        // No gravatar for this address — record the probe.
        if record(user_avatars::mark_gravatar_checked(pool, &avatar.user_id, false).await, &avatar.user_id) {
          processed += 1;
        }
      }
      Ok(Some((bytes, content_type))) => {
        let outcome = match bucket {
          Some(bucket) => mirror_gravatar(pool, bucket, &avatar.user_id, &bytes, &content_type).await,
          // No storage — existence-only fallback (source still promotes so it's observable).
          None => user_avatars::mark_gravatar_checked(pool, &avatar.user_id, true).await.map(|_| ()).map_err(Into::into),
        };
        match outcome {
          Ok(()) => {
            tracing::debug!(user_id = %avatar.user_id, mirrored = bucket.is_some(), bytes = bytes.len(), "gravatar found");
            processed += 1;
          }
          Err(e) => tracing::warn!(error = %e, user_id = %avatar.user_id, "gravatar mirror failed — will retry"),
        }
      }
      Err(e) => {
        // Transport error — leave the row unpolled so a later pass retries it (graceful failure).
        tracing::warn!(error = %e, user_id = %avatar.user_id, "gravatar fetch failed — will retry");
      }
    }
  }
  Ok(processed)
}

fn record<E: std::fmt::Display>(result: Result<u64, E>, user_id: &ids::UserId) -> bool {
  match result {
    Ok(_) => true,
    Err(e) => {
      tracing::error!(error = %e, user_id = %user_id, "failed to record probe result");
      false
    }
  }
}

/// Fetch the gravatar image. `Ok(None)` = no gravatar (404); `Ok(Some(...))` = image bytes +
/// content type. Uses `d=404` so a missing avatar is a 404, not a generated placeholder.
async fn fetch_gravatar(http: &reqwest::Client, hash: &str) -> reqwest::Result<Option<(Vec<u8>, String)>> {
  let url = format!("https://gravatar.com/avatar/{hash}?s=512&d=404");
  let resp = http.get(&url).send().await?;
  if !resp.status().is_success() {
    return Ok(None);
  }
  let content_type = resp
    .headers()
    .get("content-type")
    .and_then(|v| v.to_str().ok())
    .unwrap_or("image/png")
    .to_string();
  let bytes = resp.bytes().await?.to_vec();
  Ok(Some((bytes, content_type)))
}

/// Mirror gravatar bytes into R2 + record the `user_avatar_files` row + promote the avatar
/// (accounts.md §2). Upload happens OUTSIDE the db transaction; an orphaned object on a late
/// failure is harmless (unguessable key, retried row).
async fn mirror_gravatar(
  pool: &db::PgPool,
  bucket: &BucketClient,
  user_id: &ids::UserId,
  bytes: &[u8],
  content_type: &str,
) -> anyhow::Result<()> {
  let extension = extension_for(content_type);
  let bucket_hash = mint_bucket_hash();
  let key = object_key(ObjectKeyParts {
    folder: BucketFolder::Avatars,
    hash: &bucket_hash,
    prefix: None,
    suffix: None,
    version: None,
    extension: Some(extension),
  });

  bucket
    .upload_bytes(UploadBytesArgs { object_key: &key, bytes, content_type: Some(content_type) })
    .await
    .map_err(|e| anyhow::anyhow!("upload {key}: {e}"))?;

  let (width, height) = image::load_from_memory(bytes).map(|i| (Some(i.width() as i32), Some(i.height() as i32))).unwrap_or((None, None));
  let content_hash = hex::encode(Sha256::digest(bytes));

  let mut tx = pool.begin().await?;
  let file = user_avatar_files::create_user_avatar_file(
    &mut *tx,
    user_avatar_files::NewUserAvatarFile {
      user_id,
      original_user_avatar_file_id: None,
      avatar_source: enums::AvatarFileSource::GravatarMirror,
      bucket_hash: &bucket_hash,
      content_hash: &content_hash,
      bucket_suffix: None,
      bucket_extension: extension,
      width,
      height,
      bytes: bytes.len() as i64,
    },
  )
  .await?;
  user_avatars::set_gravatar_mirrored(&mut *tx, user_id, &file.id).await?;
  tx.commit().await?;
  Ok(())
}

/// Map a content type to the stored `bucket_extension` (leading period included).
fn extension_for(content_type: &str) -> &'static str {
  match content_type {
    t if t.contains("jpeg") || t.contains("jpg") => ".jpg",
    t if t.contains("webp") => ".webp",
    t if t.contains("gif") => ".gif",
    _ => ".png",
  }
}
