use crate::api::requests::legacy_common::{legacy_get, PageParams};
use crate::api::types::element_types::element_task_result::ElementTaskResult;
use crate::api::types::legacy_types::legacy_task::LegacyTask;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::kling_error::KlingError;

pub(crate) const PATH: &str = "/v1/general/advanced-custom-elements";
const MAX_PAGE_SIZE: u32 = 500;

#[derive(Clone, Debug)]
pub struct ListElementsArgs<'a> {
  pub api_key: &'a KlingApiKey,
  pub page: PageParams,
}

/// `GET /v1/general/advanced-custom-elements?pageNum&pageSize` — page through
/// the account's element creation tasks (newest first; `pageSize` ≤ 500).
pub async fn list_elements(args: ListElementsArgs<'_>) -> Result<Vec<LegacyTask<ElementTaskResult>>, KlingError> {
  let ListElementsArgs { api_key, page } = args;
  page.validate(MAX_PAGE_SIZE)?;
  legacy_get(api_key, PATH, &page.to_query()).await
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn documented_list_response_parses() {
    let json = r#"[
      {
        "task_id": "1",
        "task_status": "succeed",
        "task_result": {"elements": [{"element_id": 1, "element_name": "a", "status": "succeed"}]}
      },
      {"task_id": "2", "task_status": "processing"}
    ]"#;
    let tasks: Vec<LegacyTask<ElementTaskResult>> = serde_json::from_str(json).unwrap();
    assert_eq!(tasks.len(), 2);
    assert_eq!(tasks[0].task_result.as_ref().unwrap().elements.len(), 1);
    assert!(tasks[1].task_result.is_none());
  }
}
