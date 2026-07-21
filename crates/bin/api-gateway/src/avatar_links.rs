//! Avatar link building (MediaLinks pattern): the API returns **CDN URLs** derived from stored
//! bucket-path pieces — the gateway never serves asset bytes (it is not a CDN).
//!
//! `thumbnail_template` carries a `{WIDTH}` placeholder for the client to substitute:
//!   - production: Cloudflare Image Resizing — `{host}/cdn-cgi/image/width={WIDTH},quality=90/{key}`
//!   - development: no resizer in front of the bucket, so the template is just the full-image URL

use std::collections::HashMap;

use enums::AvatarSource;
use serde::Serialize;
use utoipa::ToSchema;

use bucket_paths::{object_key, BucketFolder, ObjectKeyParts};
use db_queries::{user_avatar_files, user_avatars, UserAvatarFile};
use ids::{UserAvatarFileId, UserId};

use crate::error::ApiResult;
use crate::state::AppState;

const THUMBNAIL_QUALITY: u8 = 90;

/// Links to an avatar image on the CDN.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AvatarLinks {
  /// Direct link to the full-size image.
  pub cdn_url: String,
  /// Template for sized variants — replace `{WIDTH}` with the desired pixel width. In
  /// development (no image resizer) this is simply the full image URL.
  pub thumbnail_template: String,
}

/// The avatar as APIs report it: the user's `preferred_source`, the derived effective `source`
/// (what actually displays), CDN links when an image backs it (`links` is `null` for the
/// letter/generated monograms — the client renders those itself), and the availability flags
/// the settings UI needs to offer choose/upload/remove/refresh.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AvatarInfo {
  /// What's actually displaying: the preference when satisfiable, else the monogram. Never
  /// `NotYetSet` (that's a preference, not a display).
  #[schema(value_type = String, example = "gravatar")]
  pub source: AvatarSource,
  /// The user's stated choice; `NotYetSet` until they pick (the system auto-picks meanwhile).
  #[schema(value_type = String, example = "not_yet_set")]
  pub preferred_source: AvatarSource,
  pub links: Option<AvatarLinks>,
  /// A custom upload exists (choosable even when not displaying).
  pub has_custom: bool,
  /// A mirrored gravatar exists (choosable even when not displaying).
  pub has_gravatar: bool,
  /// False while a gravatar (re-)poll is pending — the job hasn't checked yet.
  pub gravatar_checked: bool,
}

/// The default for users with no avatar row: the letter monogram.
pub fn letter_only() -> AvatarInfo {
  AvatarInfo {
    source: AvatarSource::Letter,
    preferred_source: AvatarSource::Letter,
    links: None,
    has_custom: false,
    has_gravatar: false,
    gravatar_checked: false,
  }
}

/// Effective display: the preference if a file backs it; NotYetSet auto-picks the gravatar
/// when one is mirrored, else the letter monogram (accounts.md §2).
fn effective_display(avatar: &db_queries::UserAvatar) -> (AvatarSource, Option<UserAvatarFileId>) {
  match avatar.preferred_source {
    AvatarSource::Custom if avatar.custom_file_id.is_some() => (AvatarSource::Custom, avatar.custom_file_id.clone()),
    AvatarSource::Gravatar | AvatarSource::NotYetSet if avatar.gravatar_file_id.is_some() => {
      (AvatarSource::Gravatar, avatar.gravatar_file_id.clone())
    }
    AvatarSource::Generated => (AvatarSource::Generated, None),
    _ => (AvatarSource::Letter, None),
  }
}

fn assemble(state: &AppState, avatar: &db_queries::UserAvatar, file: Option<&UserAvatarFile>) -> AvatarInfo {
  let (source, _) = effective_display(avatar);
  AvatarInfo {
    source,
    preferred_source: avatar.preferred_source,
    links: file.map(|f| links_for_file(state, f)),
    has_custom: avatar.custom_file_id.is_some(),
    has_gravatar: avatar.gravatar_file_id.is_some(),
    gravatar_checked: avatar.gravatar_checked_at.is_some(),
  }
}

/// Resolve a user's avatar into its API shape. The effective display `source` is DERIVED:
/// the preference when it's satisfiable (custom needs an upload, gravatar needs a mirror),
/// else the letter monogram (accounts.md §2).
pub async fn avatar_info(state: &AppState, user_id: &UserId) -> ApiResult<AvatarInfo> {
  let Some(avatar) = user_avatars::find_user_avatar_by_user(&state.pool, user_id).await? else {
    return Ok(letter_only());
  };
  let (_, file_id) = effective_display(&avatar);
  let file = match file_id {
    Some(file_id) => user_avatar_files::find_user_avatar_file_by_id(&state.pool, &file_id).await?,
    None => None,
  };
  Ok(assemble(state, &avatar, file.as_ref()))
}

/// Batch [`avatar_info`] — two queries total, however many users (rosters, audit actors).
/// Every requested id gets an entry; users with no avatar row resolve to the letter monogram.
pub async fn avatar_infos(state: &AppState, user_ids: &[UserId]) -> ApiResult<HashMap<UserId, AvatarInfo>> {
  if user_ids.is_empty() {
    return Ok(HashMap::new());
  }
  let avatars = user_avatars::list_user_avatars_by_user_ids(&state.pool, user_ids).await?;

  let display_file_ids: Vec<UserAvatarFileId> =
    avatars.iter().filter_map(|a| effective_display(a).1).collect();
  let files: HashMap<UserAvatarFileId, UserAvatarFile> =
    user_avatar_files::list_user_avatar_files_by_ids(&state.pool, &display_file_ids)
      .await?
      .into_iter()
      .map(|f| (f.id.clone(), f))
      .collect();

  let mut by_user: HashMap<UserId, AvatarInfo> = avatars
    .into_iter()
    .map(|a| {
      let file = effective_display(&a).1.and_then(|id| files.get(&id));
      (a.user_id.clone(), assemble(state, &a, file))
    })
    .collect();
  for id in user_ids {
    by_user.entry(id.clone()).or_insert_with(letter_only);
  }
  Ok(by_user)
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
