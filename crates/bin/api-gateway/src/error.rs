//! The gateway's error type. Every handler/service returns `Result<T, ApiError>`; the
//! `IntoResponse` impl renders a JSON envelope `{ "error": { "code", "message" } }`.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug)]
pub enum ApiError {
  BadRequest(String),
  Unauthorized(String),
  Forbidden(String),
  NotFound(String),
  /// A uniqueness/state conflict (e.g. email already in use with a different method).
  Conflict(String),
  Internal(String),
}

impl ApiError {
  fn status(&self) -> StatusCode {
    match self {
      Self::BadRequest(_) => StatusCode::BAD_REQUEST,
      Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
      Self::Forbidden(_) => StatusCode::FORBIDDEN,
      Self::NotFound(_) => StatusCode::NOT_FOUND,
      Self::Conflict(_) => StatusCode::CONFLICT,
      Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
  }

  fn code(&self) -> &'static str {
    match self {
      Self::BadRequest(_) => "bad_request",
      Self::Unauthorized(_) => "unauthorized",
      Self::Forbidden(_) => "forbidden",
      Self::NotFound(_) => "not_found",
      Self::Conflict(_) => "conflict",
      Self::Internal(_) => "internal_error",
    }
  }

  fn message(&self) -> &str {
    match self {
      Self::BadRequest(m)
      | Self::Unauthorized(m)
      | Self::Forbidden(m)
      | Self::NotFound(m)
      | Self::Conflict(m)
      | Self::Internal(m) => m,
    }
  }
}

impl IntoResponse for ApiError {
  fn into_response(self) -> Response {
    // Never leak internal details to the client; log them instead.
    let public_message = match &self {
      ApiError::Internal(m) => {
        tracing::error!("internal error: {m}");
        "internal server error".to_string()
      }
      other => other.message().to_string(),
    };
    let body = Json(json!({ "error": { "code": self.code(), "message": public_message } }));
    (self.status(), body).into_response()
  }
}

impl From<sqlx::Error> for ApiError {
  fn from(e: sqlx::Error) -> Self {
    ApiError::Internal(format!("db error: {e}"))
  }
}

pub type ApiResult<T> = Result<T, ApiError>;
