use chrono::{DateTime, Utc};

mod find_review;
mod list_reviews;
mod model_ratings;
mod remove_review;
mod save_review;
mod used_models;

pub use find_review::find_review;
pub use list_reviews::{list_reviews, ReviewPage};
pub use model_ratings::model_ratings;
pub use remove_review::remove_review;
pub use save_review::{save_review, ReviewInput};
pub use used_models::{used_models, UsageFilter};

pub struct Review {
    pub rating: i16,
    pub review_text: Option<String>,
}

pub struct PublicReview {
    pub sid: i64,
    pub text_revision: i64,
    pub model: String,
    pub rating: i16,
    pub review_text: String,
    pub updated_at: DateTime<Utc>,
}

pub struct ModelRating {
    pub model: String,
    pub rating_count: i64,
    pub rating_total: i64,
}

pub struct ReviewChange {
    pub sid: i64,
    /// Present for an insert or update. A deletion has no current text revision.
    pub text_revision: Option<i64>,
}
