//! Storage pathing (files.md §2). Rows store the *pieces* (folder, hash, prefix, suffix,
//! version, extension); the object key and CDN URL are derived with the pure functions here —
//! swapping the CDN host, bucket, or sharding scheme is a config/code change, zero rows touched.
//!
//! Key shape: `{folder}/{6 single-char shards}/{prefix}{hash}{suffix}{_vN}{ext}`, e.g.
//! `avatars/7/q/3/k/9/m/7q3k9m…4p6s_512.webp`. The 32-char lowercase Crockford `hash` is
//! **entropy** (160 bits — unguessable, collision-free), not a content hash; the key IS the
//! access token for public-CDN objects.

use std::fmt;

/// Crockford base32 alphabet, lowercase (no i, l, o, u).
const CROCKFORD: &[u8] = b"0123456789abcdefghjkmnpqrstvwxyz";

/// Top-level bucket folders (`BucketFolder` — enums.md). A new folder is a new variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BucketFolder {
  Generations,
  Uploads,
  Samples,
  Avatars,
}

impl BucketFolder {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Generations => "generations",
      Self::Uploads => "uploads",
      Self::Samples => "samples",
      Self::Avatars => "avatars",
    }
  }
}

impl fmt::Display for BucketFolder {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(self.as_str())
  }
}

/// Mint a fresh 32-char lowercase Crockford entropy hash for `bucket_hash` columns.
pub fn mint_bucket_hash() -> String {
  use rand::Rng;
  let mut rng = rand::thread_rng();
  (0..32).map(|_| CROCKFORD[rng.gen_range(0..CROCKFORD.len())] as char).collect()
}

/// Derive the object key from a row's stored pieces (files.md §2).
pub fn object_key(
  folder: BucketFolder,
  hash: &str,
  prefix: Option<&str>,
  suffix: Option<&str>,
  version: Option<u32>,
  extension: Option<&str>,
) -> String {
  let shards: Vec<String> = hash.chars().take(6).map(String::from).collect();
  let filename = format!(
    "{}{}{}{}{}",
    prefix.unwrap_or(""),
    hash,
    suffix.unwrap_or(""),
    version.map(|v| format!("_v{v}")).unwrap_or_default(),
    extension.unwrap_or(""),
  );
  format!("{}/{}/{}", folder.as_str(), shards.join("/"), filename)
}

/// Derive the public CDN URL for an object key. The host comes from config, never a column.
pub fn cdn_url(cdn_base_url: &str, key: &str) -> String {
  format!("{}/{}", cdn_base_url.trim_end_matches('/'), key)
}

#[cfg(test)]
mod tests {
  use super::*;

  const HASH: &str = "7q3k9m2r5t8x1c4f7j0v2b5d8g1n4p6s";

  #[test]
  fn full_pieces_render_the_documented_key() {
    let key = object_key(
      BucketFolder::Generations,
      HASH,
      Some("foundation_"),
      Some("_seedance2p5"),
      Some(4),
      Some(".mp4"),
    );
    assert_eq!(key, "generations/7/q/3/k/9/m/foundation_7q3k9m2r5t8x1c4f7j0v2b5d8g1n4p6s_seedance2p5_v4.mp4");
  }

  #[test]
  fn avatar_key_uses_suffix_and_no_version() {
    let key = object_key(BucketFolder::Avatars, HASH, None, Some("_512"), None, Some(".webp"));
    assert_eq!(key, "avatars/7/q/3/k/9/m/7q3k9m2r5t8x1c4f7j0v2b5d8g1n4p6s_512.webp");
  }

  #[test]
  fn bare_pieces_render_bare_key() {
    let key = object_key(BucketFolder::Uploads, HASH, None, None, None, None);
    assert_eq!(key, "uploads/7/q/3/k/9/m/7q3k9m2r5t8x1c4f7j0v2b5d8g1n4p6s");
  }

  #[test]
  fn minted_hashes_are_32_crockford_chars_and_unique() {
    let a = mint_bucket_hash();
    let b = mint_bucket_hash();
    assert_eq!(a.len(), 32);
    assert!(a.bytes().all(|c| CROCKFORD.contains(&c)));
    assert_ne!(a, b);
  }

  #[test]
  fn cdn_url_joins_without_double_slash() {
    assert_eq!(cdn_url("https://cdn.foundation.ai/", "avatars/a/b.png"), "https://cdn.foundation.ai/avatars/a/b.png");
    assert_eq!(cdn_url("https://cdn.foundation.ai", "avatars/a/b.png"), "https://cdn.foundation.ai/avatars/a/b.png");
  }
}
