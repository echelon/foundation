use std::sync::Once;

static INIT: Once = Once::new();

/// Initialize tracing so live-API tests print request/response details. Idempotent.
#[cfg(test)]
pub fn setup_test_logging() {
  INIT.call_once(|| {
    let _ = tracing_subscriber::fmt().with_test_writer().with_max_level(tracing::Level::INFO).try_init();
  });
}
