use axum::{
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{header, StatusCode},
    routing::get,
    Extension, Json, Router,
};
use chrono::{DateTime, Utc};
use db_queries::{
    model_review_briefs,
    model_reviews::{self as storage, ModelRating, ReviewInput, ReviewPage},
};
use serde::{Deserialize, Serialize};

use crate::{
    error::{ApiError, ApiResult},
    gateway_demo::ScopedModels,
    middleware::AuthContext,
    state::AppState,
};

const PAGE_SIZE: usize = 20;
const MAX_TEXT_BYTES: usize = 1200;
pub(crate) const MIN_QUALITY_RATINGS: i64 = 10;

pub(crate) fn router(models: Option<ScopedModels>) -> Router<AppState> {
    Router::new()
        .route("/v1/models/:slug/reviews", get(list))
        .route("/v1/models/:slug/reviews/me", get(mine).put(save).delete(remove))
        .layer(DefaultBodyLimit::max(8 * 1024))
        .layer(Extension(models))
}

#[derive(Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub(crate) struct ListQuery {
    /// Use the previous response's next_before value.
    before: Option<i64>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct Reviews {
    model: String,
    rating_count: i64,
    average_rating: Option<f64>,
    reviews: Vec<PublicReview>,
    next_before: Option<i64>,
}

#[derive(Serialize, utoipa::ToSchema)]
struct PublicReview {
    rating: i16,
    review_text: String,
    updated_at: DateTime<Utc>,
}

#[derive(Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
struct MyReview {
    #[schema(minimum = 1, maximum = 10)]
    rating: i16,
    /// Optional public text, at most 1200 UTF-8 bytes.
    review_text: Option<String>,
}

impl From<storage::Review> for MyReview {
    fn from(row: storage::Review) -> Self {
        Self {
            rating: row.rating,
            review_text: row.review_text,
        }
    }
}

pub(crate) fn average(rating: &ModelRating) -> f64 {
    rating.rating_total as f64 / rating.rating_count as f64
}

fn check_model(models: Option<ScopedModels>, slug: &str) -> ApiResult<()> {
    if slug != "auto"
        && models.is_some_and(|models| models.contains_public(slug))
    {
        Ok(())
    } else {
        Err(ApiError::NotFound("model not found in the gateway catalog".into()))
    }
}

#[utoipa::path(
    get, path = "/v1/models/{slug}/reviews", operation_id = "listModelReviews", tag = "catalog",
    params(("slug" = String, Path, description = "Canonical model slug"), ListQuery),
    responses(
        (status = 200, description = "Current community rating and a page of anonymous written reviews.", body = Reviews),
        (status = 404, description = "Model not found in the gateway catalog."),
    ),
)]
pub(crate) async fn list(
    State(state): State<AppState>,
    Extension(models): Extension<Option<ScopedModels>>,
    Path(slug): Path<String>,
    Query(query): Query<ListQuery>,
) -> ApiResult<([(header::HeaderName, &'static str); 2], Json<Reviews>)> {
    check_model(models, &slug)?;
    if query.before.is_some_and(|cursor| cursor <= 0) {
        return Err(ApiError::BadRequest("invalid review cursor".into()));
    }

    let models = std::slice::from_ref(&slug);
    let mut tx = state.pool.begin_with("BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY").await?;
    let rating = storage::model_ratings(&mut *tx, models).await?.pop();
    let mut rows = storage::list_reviews(
        &mut *tx,
        ReviewPage { models, before: query.before, limit_per_model: PAGE_SIZE as i64 + 1 },
    )
    .await?;
    tx.commit().await?;
    let next_before = if rows.len() > PAGE_SIZE {
        rows.pop();
        rows.last().map(|row| row.sid)
    } else {
        None
    };
    Ok((
        [
            (header::CACHE_CONTROL, "no-store"),
            (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"),
        ],
        Json(Reviews {
            model: slug,
            rating_count: rating.as_ref().map_or(0, |row| row.rating_count),
            average_rating: rating.as_ref().map(average),
            reviews: rows.into_iter().map(|row| PublicReview {
                rating: row.rating,
                review_text: row.review_text,
                updated_at: row.updated_at,
            }).collect(),
            next_before,
        }),
    ))
}

async fn mine(
    State(state): State<AppState>,
    Extension(models): Extension<Option<ScopedModels>>,
    auth: AuthContext,
    Path(slug): Path<String>,
) -> ApiResult<([(header::HeaderName, &'static str); 1], Json<Option<MyReview>>)> {
    check_model(models, &slug)?;
    let row = storage::find_review(&state.pool, &slug, &auth.user.id).await?;
    Ok(([(header::CACHE_CONTROL, "no-store")], Json(row.map(MyReview::from))))
}

async fn save(
    State(state): State<AppState>,
    Extension(models): Extension<Option<ScopedModels>>,
    auth: AuthContext,
    Path(slug): Path<String>,
    Json(mut review): Json<MyReview>,
) -> ApiResult<Json<MyReview>> {
    check_model(models, &slug)?;
    require_reviewer(&auth)?;
    if !(1..=10).contains(&review.rating) {
        return Err(ApiError::BadRequest("rating must be an integer from 1 to 10".into()));
    }
    review.review_text = review.review_text
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty());
    if review.review_text.as_ref().is_some_and(|text| {
        text.len() > MAX_TEXT_BYTES || text.chars().any(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t'))
    }) {
        return Err(ApiError::BadRequest("review must be plain text of at most 1200 UTF-8 bytes".into()));
    }
    let mut tx = state.pool.begin().await?;
    let changed = storage::save_review(&mut *tx, ReviewInput {
        model: &slug,
        user: &auth.user.id,
        rating: review.rating,
        review_text: review.review_text.as_deref(),
    }).await?;
    if let Some(changed) = changed {
        model_review_briefs::mark_dirty(&mut *tx, &slug, changed).await?;
    }
    tx.commit().await?;
    Ok(Json(review))
}

async fn remove(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(slug): Path<String>,
) -> ApiResult<StatusCode> {
    require_reviewer(&auth)?;
    let mut tx = state.pool.begin().await?;
    if let Some(changed) = storage::remove_review(&mut *tx, &slug, &auth.user.id).await? {
        model_review_briefs::mark_dirty(&mut *tx, &slug, changed).await?;
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

fn require_reviewer(auth: &AuthContext) -> ApiResult<()> {
    if auth.is_impersonation() {
        return Err(ApiError::Forbidden("review changes require your own session".into()));
    }
    Ok(())
}
