use crate::api::requests::legacy_common::{legacy_get, PageParams};
use crate::api::types::legacy_types::legacy_task::LegacyTask;
use crate::api::types::voice_types::voice_task_result::VoiceTaskResult;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::kling_error::KlingError;

pub(crate) const PATH: &str = "/v1/general/custom-voices";
/// Voice lists allow up to 1000 per page (elements cap at 500).
const MAX_PAGE_SIZE: u32 = 1000;

#[derive(Clone, Debug)]
pub struct ListVoicesArgs<'a> {
  pub api_key: &'a KlingApiKey,
  pub page: PageParams,
}

/// `GET /v1/general/custom-voices?pageNum&pageSize` — page through the
/// account's voice creation tasks.
pub async fn list_voices(args: ListVoicesArgs<'_>) -> Result<Vec<LegacyTask<VoiceTaskResult>>, KlingError> {
  let ListVoicesArgs { api_key, page } = args;
  page.validate(MAX_PAGE_SIZE)?;
  legacy_get(api_key, PATH, &page.to_query()).await
}
