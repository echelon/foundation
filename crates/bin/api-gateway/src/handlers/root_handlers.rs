//! `GET /` — a friendly landing page for humans who wander into the API host: hello, links
//! (website / Discord / GitHub), a hiring note, and a tiny orb-popping canvas toy. The page is
//! one self-contained HTML file (inline CSS + JS, no external assets), embedded into the binary
//! at build time via `include_str!` — no filesystem access at runtime.

use axum::response::Html;

/// The whole page, baked into the binary at compile time.
const INDEX_HTML: &str = include_str!("../../static/index.html");

/// `GET /`
pub async fn index() -> Html<&'static str> {
  Html(INDEX_HTML)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn embedded_page_is_self_contained_and_on_message() {
    // Structure: one document with inline style + script, no external fetches.
    assert!(INDEX_HTML.starts_with("<!doctype html>"));
    assert!(INDEX_HTML.contains("<style>") && INDEX_HTML.contains("<script>"));
    for external in ["src=\"http", "href=\"http://cdn", "@import", "fetch("] {
      assert!(!INDEX_HTML.contains(external), "unexpected external reference: {external}");
    }
    // Content: greeting, the three links, and the hiring note.
    assert!(INDEX_HTML.contains("Hello, human"));
    for link in ["foundation.ai", "discord.gg", "github.com"] {
      assert!(INDEX_HTML.contains(link), "missing link: {link}");
    }
    assert!(INDEX_HTML.contains("Rust</span> engineers"));
  }
}
