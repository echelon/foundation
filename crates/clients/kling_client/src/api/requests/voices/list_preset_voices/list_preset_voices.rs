use crate::api::requests::legacy_common::{legacy_get, PageParams};
use crate::api::types::legacy_types::legacy_task::LegacyTask;
use crate::api::types::voice_types::voice_task_result::VoiceTaskResult;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::kling_error::KlingError;

pub(crate) const PATH: &str = "/v1/general/presets-voices";
const MAX_PAGE_SIZE: u32 = 1000;

#[derive(Clone, Debug)]
pub struct ListPresetVoicesArgs<'a> {
  pub api_key: &'a KlingApiKey,
  pub page: PageParams,
}

/// `GET /v1/general/presets-voices?pageNum&pageSize` — page through Kling's
/// official preset voice library (same shape as the custom list, minus
/// deduction fields; `owned_by` is `"kling"`).
pub async fn list_preset_voices(args: ListPresetVoicesArgs<'_>) -> Result<Vec<LegacyTask<VoiceTaskResult>>, KlingError> {
  let ListPresetVoicesArgs { api_key, page } = args;
  page.validate(MAX_PAGE_SIZE)?;
  legacy_get(api_key, PATH, &page.to_query()).await
}
