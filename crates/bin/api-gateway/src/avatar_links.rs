//! Avatar link building (MediaLinks pattern): the API returns **CDN URLs** derived from stored
//! bucket-path pieces — the gateway never serves asset bytes (it is not a CDN).
//!
//! `thumbnail_template` carries a `{WIDTH}` placeholder for the client to substitute:
//!   - production: Cloudflare Image Resizing — `{host}/cdn-cgi/image/width={WIDTH},quality=90/{key}`
//!   - development: no resizer in front of the bucket, so the template is just the full-image URL

use serde::Serialize;

use bucket_paths::{object_key, BucketFolder, ObjectKeyParts};
use db_queries::{user_avatar_files, user_avatars, UserAvatarFile};
use ids::{UserAvatarFileId, UserId};

use crate::error::ApiResult;
use crate::state::AppState;

const THUMBNAIL_QUALITY: u8 = 90;

/// Links to an avatar image on the CDN.
#[derive(Debug, Clone, Serialize)]
pub struct AvatarLinks {
  /// Direct link to the full-size image.
  pub cdn_url: String,
  /// Template for sized variants — replace `{WIDTH}` with the desired pixel width. In
  /// development (no image resizer) this is simply the full image URL.
  pub thumbnail_template: String,
}

/// The avatar as APIs report it: the display `source`, CDN links when an image backs it
/// (`links` is `null` for letter/generated monograms — the client renders those itself), and the
/// state flags the settings UI needs to offer upload/remove/dismiss/re-enable/refresh.
#[derive(Debug, Clone, Serialize)]
pub struct AvatarInfo {
  pub source: String,
  pub links: Option<AvatarLinks>,
  /// A custom upload exists (may or may not be displaying).
  pub has_custom: bool,
  /// A mirrored gravatar exists (may be dismissed or outranked by a custom upload).
  pub has_gravatar: bool,
  pub is_gravatar_dismissed: bool,
  /// False while a gravatar (re-)poll is pending — the job hasn't checked yet.
  pub gravatar_checked: bool,
}

/// Resolve a user's avatar into its API shape (accounts.md §2 display precedence).
pub async fn avatar_info(state: &AppState, user_id: &UserId) -> ApiResult<AvatarInfo> {
  let Some(avatar) = user_avatars::find_user_avatar_by_user(&state.pool, user_id).await? else {
    return Ok(AvatarInfo {
      source: "letter".into(),
      links: None,
      has_custom: false,
      has_gravatar: false,
      is_gravatar_dismissed: false,
      gravatar_checked: false,
    });
  };

  // Which file backs the display source?
  let file_id: Option<UserAvatarFileId> = match avatar.source.as_str() {
    "custom" => avatar.custom_file_id.clone(),
    "gravatar" if !avatar.is_gravatar_dismissed => avatar.gravatar_file_id.clone(),
    _ => None,
  };

  let links = match file_id {
    Some(file_id) => user_avatar_files::find_user_avatar_file_by_id(&state.pool, &file_id)
      .await?
      .map(|file| links_for_file(state, &file)),
    None => None,
  };
  Ok(AvatarInfo {
    source: avatar.source,
    links,
    has_custom: avatar.custom_file_id.is_some(),
    has_gravatar: avatar.gravatar_file_id.is_some(),
    is_gravatar_dismissed: avatar.is_gravatar_dismissed,
    gravatar_checked: avatar.gravatar_checked_at.is_some(),
  })
}

/// Build the CDN links for one stored avatar file.
pub fn links_for_file(state: &AppState, file: &UserAvatarFile) -> AvatarLinks {
  let key = object_key(ObjectKeyParts {
    folder: BucketFolder::Avatars,
    hash: &file.bucket_hash,
    prefix: None,
    suffix: file.bucket_suffix.as_deref(),
    version: None,
    extension: Some(&file.bucket_extension),
  });
  // `cdn_base_url` is normalized at config load (absolute, scheme guaranteed, no trailing
  // slash) — every URL we emit here is a FULL URL, never a relative path.
  let host = state.cdn_base_url.as_str();
  let cdn_url = format!("{host}/{key}");

  let thumbnail_template = match state.environment {
    // Production: Cloudflare Image Resizing in front of the bucket.
    config::Environment::Prod => {
      format!("{host}/cdn-cgi/image/width={{WIDTH}},quality={THUMBNAIL_QUALITY}/{key}")
    }
    // Development / test / staging: a bare bucket (R2 public dev URL or minio) with no
    // resizer — the template is simply the full image.
    config::Environment::Dev | config::Environment::Test | config::Environment::Stage => cdn_url.clone(),
  };

  AvatarLinks { cdn_url, thumbnail_template }
}
