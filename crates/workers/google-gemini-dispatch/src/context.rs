//! Everything a dispatch flow needs — one shared, cloneable context.

use google_gemini_api_client::creds::google_gemini_api_key::GoogleGeminiApiKey;

#[derive(Clone)]
pub struct DispatchContext {
  pub pool: sqlx::PgPool,
  pub api_key: GoogleGeminiApiKey,
  /// Output/input byte storage (R2). `None` = ingest fails loudly per job, the
  /// process still boots (matches the gateway's posture).
  pub bucket: Option<bucket_client::BucketClient>,
  /// For fetching OUR CDN inputs before re-uploading them to Google.
  pub http: reqwest::Client,
}
