//! `GET /v1/models` — the public model catalog.

use axum::Json;

/// The full public catalog (static, code-defined — catalog crate).
#[utoipa::path(
  get, path = "/v1/models",
  tag = "catalog",
  responses((status = 200, description = "All models", body = Object))
)]
pub async fn list_models_handler() -> Json<&'static [catalog::ModelSpec]> {
  Json(catalog::MODELS)
}
