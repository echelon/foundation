//! `GET /` — the embedded landing page (see `static/index.html`).

use axum::response::Html;

const INDEX_HTML: &str = include_str!("../../../static/index.html");

/// The friendly HTML landing page for humans who wander into the API host.
#[utoipa::path(
  get, path = "/",
  tag = "meta",
  responses((status = 200, description = "The landing page", content_type = "text/html"))
)]
pub async fn index_handler() -> Html<&'static str> {
  Html(INDEX_HTML)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn embedded_page_is_self_contained_and_on_message() {
    assert!(INDEX_HTML.starts_with("<!doctype html>"));
    assert!(INDEX_HTML.contains("<style>") && INDEX_HTML.contains("<script>"));
    for external in ["src=\"http", "href=\"http://cdn", "@import", "fetch("] {
      assert!(!INDEX_HTML.contains(external), "unexpected external reference: {external}");
    }
    assert!(INDEX_HTML.contains("Ya ha ha! You found me!"));
    assert!(INDEX_HTML.contains(".~*~^~v~"), "missing k1's frond umbrella");
    for variant in ["const k1", "const k2", "const k3"] {
      assert!(INDEX_HTML.contains(variant), "missing sprite {variant}");
    }
  }
}
