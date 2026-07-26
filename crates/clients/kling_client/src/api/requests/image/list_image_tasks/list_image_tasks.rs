use crate::api::requests::legacy_common::{legacy_get, PageParams};
use crate::api::types::image_types::image_task_result::ImageTaskResult;
use crate::api::types::legacy_types::legacy_task::LegacyTask;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::kling_error::KlingError;

pub(crate) const PATH: &str = "/v1/images/generations";
const MAX_PAGE_SIZE: u32 = 500;

#[derive(Clone, Debug)]
pub struct ListImageTasksArgs<'a> {
  pub api_key: &'a KlingApiKey,
  pub page: PageParams,
}

/// `GET /v1/images/generations?pageNum&pageSize` — page through the
/// account's Kling Image 3.0 generation tasks.
pub async fn list_image_tasks(args: ListImageTasksArgs<'_>) -> Result<Vec<LegacyTask<ImageTaskResult>>, KlingError> {
  let ListImageTasksArgs { api_key, page } = args;
  page.validate(MAX_PAGE_SIZE)?;
  legacy_get(api_key, PATH, &page.to_query()).await
}
