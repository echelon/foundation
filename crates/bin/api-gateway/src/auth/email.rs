//! Email canonicalization (accounts.md §2). Three tiers, two jobs:
//!   - [`canonicalize`]        — conservative (trim + lowercase). The UNIQUE login/collision key.
//!   - [`canonicalize_clean`]  — aggressive (gmail dot/plus stripping). An abuse/dupe signal
//!                               only; NEVER a login key.

/// Conservative canonicalization: trim + lowercase. This is the login/collision key.
pub fn canonicalize(email: &str) -> String {
  email.trim().to_lowercase()
}

/// Aggressive canonicalization for dedup/abuse signals: on top of [`canonicalize`], fold
/// `googlemail.com` → `gmail.com`, drop everything after a `+` in the local part, and remove
/// dots from gmail local parts (which Gmail ignores). Never used as a login key.
pub fn canonicalize_clean(email: &str) -> String {
  let base = canonicalize(email);
  let Some((local, domain)) = base.split_once('@') else {
    return base;
  };
  let domain = if domain == "googlemail.com" { "gmail.com" } else { domain };

  // strip +tag
  let local = local.split('+').next().unwrap_or(local);
  // gmail ignores dots
  let local = if domain == "gmail.com" { local.replace('.', "") } else { local.to_string() };

  format!("{local}@{domain}")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn canonicalize_trims_and_lowercases() {
    assert_eq!(canonicalize("  Alice@Example.COM "), "alice@example.com");
  }

  #[test]
  fn clean_strips_gmail_dots_and_plus() {
    assert_eq!(canonicalize_clean("A.L.i.c.e+promo@gmail.com"), "alice@gmail.com");
    assert_eq!(canonicalize_clean("bob+x@googlemail.com"), "bob@gmail.com");
  }

  #[test]
  fn clean_leaves_non_gmail_dots() {
    assert_eq!(canonicalize_clean("a.b+tag@example.com"), "a.b@example.com");
  }

  #[test]
  fn clean_and_canonical_differ_only_where_expected() {
    assert_eq!(canonicalize("A.B@example.com"), "a.b@example.com");
    assert_eq!(canonicalize_clean("A.B@example.com"), "a.b@example.com");
  }
}
