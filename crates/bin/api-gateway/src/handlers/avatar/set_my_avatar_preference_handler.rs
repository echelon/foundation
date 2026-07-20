//! `PUT /v1/auth/avatar/preference` — choose what displays.

use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use utoipa::ToSchema;

use db_queries::user_avatars;
use enums::AvatarSource;

use super::support::{avatar_response, AvatarResponse};
use crate::error::{ApiError, ApiResult};
use crate::middleware::AuthContext;
use crate::state::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct AvatarPreferenceBody {
  /// Typed via `enums::AvatarSource` — unknown values are rejected at deserialization.
  #[schema(value_type = String, example = "gravatar")]
  pub source: AvatarSource,
}

/// `custom` requires an upload to exist; `gravatar` is allowed even before a mirror lands (the
/// monogram shows until it does).
#[utoipa::path(
  put, path = "/v1/auth/avatar/preference",
  tag = "avatar",
  security(("session_cookie" = [])),
  request_body = AvatarPreferenceBody,
  responses(
    (status = 200, description = "Preference stored; the resulting avatar", body = AvatarResponse),
    (status = 400, description = "Unchoosable source, or `custom` without an upload")
  )
)]
pub async fn set_my_avatar_preference_handler(
  State(state): State<AppState>,
  auth: AuthContext,
  Json(body): Json<AvatarPreferenceBody>,
) -> ApiResult<Json<AvatarResponse>> {
  match body.source {
    AvatarSource::Custom => {
      let has_custom = user_avatars::find_user_avatar_by_user(&state.pool, &auth.user.id)
        .await?
        .is_some_and(|a| a.custom_file_id.is_some());
      if !has_custom {
        return Err(ApiError::BadRequest("upload a photo before choosing it".into()));
      }
    }
    AvatarSource::Gravatar | AvatarSource::Letter => {}
    AvatarSource::NotYetSet | AvatarSource::Generated => {
      return Err(ApiError::BadRequest(format!("{} is not a choosable source", body.source)));
    }
  }
  user_avatars::set_preferred_source(&state.pool, &auth.user.id, body.source).await?;
  avatar_response(&state, &auth).await
}
