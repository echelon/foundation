use crate::api::requests::legacy_common::{legacy_get, PageParams};
use crate::api::types::audio_types::audio_task_result::AudioTaskResult;
use crate::api::types::legacy_types::legacy_task::LegacyTask;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::kling_error::KlingError;

pub(crate) const PATH: &str = "/v1/audio/text-to-audio";
const MAX_PAGE_SIZE: u32 = 500;

#[derive(Clone, Debug)]
pub struct ListTextToAudioTasksArgs<'a> {
  pub api_key: &'a KlingApiKey,
  pub page: PageParams,
}

/// `GET /v1/audio/text-to-audio?pageNum&pageSize` — page through the
/// account's text-to-audio generation tasks.
pub async fn list_text_to_audio_tasks(
  args: ListTextToAudioTasksArgs<'_>,
) -> Result<Vec<LegacyTask<AudioTaskResult>>, KlingError> {
  let ListTextToAudioTasksArgs { api_key, page } = args;
  page.validate(MAX_PAGE_SIZE)?;
  legacy_get(api_key, PATH, &page.to_query()).await
}
