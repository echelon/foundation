use crate::api::requests::legacy_common::{legacy_get, PageParams};
use crate::api::types::element_types::element_task_result::ElementTaskResult;
use crate::api::types::legacy_types::legacy_task::LegacyTask;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::kling_error::KlingError;

pub(crate) const PATH: &str = "/v1/general/advanced-presets-elements";
const MAX_PAGE_SIZE: u32 = 500;

#[derive(Clone, Debug)]
pub struct ListPresetElementsArgs<'a> {
  pub api_key: &'a KlingApiKey,
  pub page: PageParams,
}

/// `GET /v1/general/advanced-presets-elements?pageNum&pageSize` — page
/// through Kling's official preset element library (same task shape as the
/// custom list, minus deduction fields; `owned_by` is `"kling"`).
pub async fn list_preset_elements(
  args: ListPresetElementsArgs<'_>,
) -> Result<Vec<LegacyTask<ElementTaskResult>>, KlingError> {
  let ListPresetElementsArgs { api_key, page } = args;
  page.validate(MAX_PAGE_SIZE)?;
  legacy_get(api_key, PATH, &page.to_query()).await
}
