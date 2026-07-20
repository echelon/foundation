//! Random placeholder business names — `"Adjective Noun"` (e.g. "Radiant Compass"), minted at
//! account creation and shown until an owner/admin sets a real `business_name`.
//!
//! Technique borrowed from ArtCraft's username generator: word lists embedded at compile time,
//! one trimmed non-comment line per word. The animals list is deliberately NOT included —
//! businesses may find it insulting. The lists are copies (originals live in ArtCraft's
//! `includes/binary_includes/usernames/atoms/`); trim freely.

use std::sync::LazyLock;

use rand::seq::SliceRandom;

const ADJECTIVES: &str = include_str!("../assets/business_names/business_name_adjectives.txt");
const NOUNS: &str = include_str!("../assets/business_names/business_name_nouns.txt");

static ADJECTIVE_LIST: LazyLock<Vec<&'static str>> = LazyLock::new(|| words(ADJECTIVES));
static NOUN_LIST: LazyLock<Vec<&'static str>> = LazyLock::new(|| words(NOUNS));

/// Non-empty, non-comment lines, trimmed.
fn words(raw: &'static str) -> Vec<&'static str> {
  raw
    .lines()
    .map(str::trim)
    .filter(|l| !l.is_empty() && !l.starts_with('#'))
    .collect()
}

fn title_case(word: &str) -> String {
  let mut chars = word.chars();
  match chars.next() {
    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    None => String::new(),
  }
}

/// A random `"Adjective Noun"` placeholder business name. Infallible.
pub fn random_business_name() -> String {
  let mut rng = rand::thread_rng();
  let adjective = ADJECTIVE_LIST.choose(&mut rng).copied().unwrap_or("acclaimed");
  let noun = NOUN_LIST.choose(&mut rng).copied().unwrap_or("venture");
  format!("{} {}", title_case(adjective), title_case(noun))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn lists_are_populated_and_comment_free() {
    assert!(ADJECTIVE_LIST.len() > 100, "adjective list looks truncated");
    assert!(NOUN_LIST.len() > 100, "noun list looks truncated");
    for w in ADJECTIVE_LIST.iter().chain(NOUN_LIST.iter()) {
      assert!(!w.starts_with('#') && !w.contains(char::is_whitespace), "bad atom: {w:?}");
    }
  }

  #[test]
  fn generates_capitalized_alphabetic_word_pairs() {
    // A business name, not a username: "Capitalized Capitalized", letters only — no digits,
    // underscores, or separators beyond the single space.
    for _ in 0..50 {
      let name = random_business_name();
      let parts: Vec<&str> = name.split(' ').collect();
      assert_eq!(parts.len(), 2, "expected 'Adjective Noun', got {name:?}");
      for part in parts {
        assert!(part.chars().next().unwrap().is_uppercase(), "not capitalized: {name:?}");
        assert!(part.chars().all(char::is_alphabetic), "non-letter in {name:?}");
      }
    }
  }
}
