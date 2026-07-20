//! `GET /v1/models/{slug}` — one catalog model.

use axum::extract::Path;
use axum::http::StatusCode;
use axum::Json;

#[utoipa::path(
  get, path = "/v1/models/{slug}",
  tag = "catalog",
  params(("slug" = String, Path, description = "Model slug, e.g. `seedance-2-0`")),
  responses(
    (status = 200, description = "The model", body = Object),
    (status = 404, description = "Unknown slug")
  )
)]
pub async fn get_model_handler(Path(slug): Path<String>) -> Result<Json<&'static catalog::ModelSpec>, StatusCode> {
  catalog::by_slug(&slug).map(Json).ok_or(StatusCode::NOT_FOUND)
}
