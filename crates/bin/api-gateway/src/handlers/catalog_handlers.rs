//! Health + public catalog handlers.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct Health {
  pub status: &'static str,
  pub database: &'static str,
  pub models: usize,
}

pub async fn health(State(state): State<AppState>) -> Json<Health> {
  let database = if db::ping(&state.pool).await { "up" } else { "down" };
  Json(Health { status: "ok", database, models: catalog::MODELS.len() })
}

/// `GET /v1/models` — the public catalog.
pub async fn list_models() -> Json<&'static [catalog::ModelSpec]> {
  Json(catalog::MODELS)
}

/// `GET /v1/models/{slug}`
pub async fn get_model(Path(slug): Path<String>) -> Result<Json<&'static catalog::ModelSpec>, StatusCode> {
  catalog::by_slug(&slug).map(Json).ok_or(StatusCode::NOT_FOUND)
}
