//! Password hashing — argon2id (accounts.md §2). Hashes are self-describing (the salt and
//! params live inside the encoded string), so verification needs only the stored hash.

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;

/// Hash a plaintext password with argon2id + a random salt. Returns the PHC-encoded string.
pub fn hash_password(plaintext: &str) -> Result<String, String> {
  let salt = SaltString::generate(&mut OsRng);
  Argon2::default()
    .hash_password(plaintext.as_bytes(), &salt)
    .map(|h| h.to_string())
    .map_err(|e| format!("argon2 hash failed: {e}"))
}

/// Verify a plaintext against a stored argon2 hash. Returns false on any mismatch/parse error
/// (never leaks *why*).
pub fn verify_password(plaintext: &str, stored_hash: &str) -> bool {
  match PasswordHash::new(stored_hash) {
    Ok(parsed) => Argon2::default().verify_password(plaintext.as_bytes(), &parsed).is_ok(),
    Err(_) => false,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn hash_then_verify_roundtrip() {
    let hash = hash_password("correct horse battery staple").unwrap();
    assert!(hash.starts_with("$argon2id$"), "expected argon2id hash, got {hash}");
    assert!(verify_password("correct horse battery staple", &hash));
  }

  #[test]
  fn wrong_password_fails() {
    let hash = hash_password("s3cret").unwrap();
    assert!(!verify_password("wrong", &hash));
  }

  #[test]
  fn garbage_hash_does_not_panic() {
    assert!(!verify_password("x", "not-a-real-hash"));
  }

  #[test]
  fn salts_differ_between_hashes() {
    assert_ne!(hash_password("same").unwrap(), hash_password("same").unwrap());
  }
}
