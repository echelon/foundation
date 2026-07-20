use std::fmt;

/// A normalized object key for the bucket.
///
/// Leading slashes are stripped: S3/R2 treat a leading `/` as an empty leading path segment,
/// which silently produces surprising keys (e.g. `/foo.png` becomes an object literally named
/// `/foo.png`). Normalizing here keeps stored keys consistent regardless of caller formatting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectKey(String);

impl ObjectKey {
  /// Build a normalized object key from any string-like value.
  pub fn new(raw: impl AsRef<str>) -> Self {
    ObjectKey(raw.as_ref().trim_start_matches('/').to_string())
  }

  /// The normalized key as a string slice.
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

impl fmt::Display for ObjectKey {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&self.0)
  }
}

impl AsRef<str> for ObjectKey {
  fn as_ref(&self) -> &str {
    &self.0
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn strips_leading_slashes() {
    assert_eq!(ObjectKey::new("/foo/bar.png").as_str(), "foo/bar.png");
    assert_eq!(ObjectKey::new("///a").as_str(), "a");
    assert_eq!(ObjectKey::new("already/clean.txt").as_str(), "already/clean.txt");
  }

  #[test]
  fn accepts_string_and_str() {
    assert_eq!(ObjectKey::new(String::from("x.bin")).as_str(), "x.bin");
    assert_eq!(ObjectKey::new("x.bin").as_str(), "x.bin");
  }
}
