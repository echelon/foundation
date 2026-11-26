use chrono::{DateTime, Utc};
use serde_json::Value;

mod claim_refresh;
mod list_current;
mod mark_dirty;
mod publish;
mod save_baseline;
mod seed_models;

pub use claim_refresh::claim_refresh;
pub use list_current::list_current;
pub use mark_dirty::mark_dirty;
pub use publish::{publish, BriefUpdate};
pub use save_baseline::save_baseline;
pub use seed_models::seed_models;

pub struct Refresh {
    pub model: String,
    pub revision: i64,
    pub refresh_after: DateTime<Utc>,
    pub baseline: Option<Value>,
    pub community: Option<Value>,
    pub review_sample: Value,
}

pub struct Brief {
    pub model: String,
    pub rating_count: i64,
    pub rating_total: i64,
    pub written_reviews_sampled: i32,
    pub strengths: Vec<String>,
    pub weaknesses: Vec<String>,
    pub refreshed_at: DateTime<Utc>,
    pub baseline: Option<Value>,
    pub community: Option<Value>,
}
