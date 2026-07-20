//! `GET /health` — liveness + database reachability.

use axum::extract::State;
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
  #[schema(example = "ok")]
  pub status: &'static str,
  #[schema(example = "up")]
  pub database: &'static str,
  /// Number of models in the catalog.
  pub models: usize,
}

#[utoipa::path(
  get, path = "/health",
  tag = "meta",
  responses((status = 200, description = "Service health", body = HealthResponse))
)]
pub async fn health_handler(State(state): State<AppState>) -> Json<HealthResponse> {
  let database = if db::ping(&state.pool).await { "up" } else { "down" };
  Json(HealthResponse { status: "ok", database, models: catalog::MODELS.len() })
}
