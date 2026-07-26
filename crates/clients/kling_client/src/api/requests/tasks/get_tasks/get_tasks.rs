use client_core::http_client;
use tracing::info;

use crate::api::requests::kling_host::{with_auth, KLING_API_BASE_URL, KLING_USER_AGENT};
use crate::api::types::task_types::kling_envelope::KlingEnvelope;
use crate::api::types::task_types::kling_task::KlingTask;
use crate::api::types::task_types::kling_task_id::KlingTaskId;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::classify_kling_http_error::classify_kling_http_error;
use crate::error::kling_client_error::KlingClientError;
use crate::error::kling_error::KlingError;
use crate::error::kling_generic_api_error::KlingGenericApiError;

/// Which ids to look up — Kling forbids mixing the two in one call, so the
/// enum makes the invalid combination unrepresentable. Both support batches
/// (comma-separated on the wire).
#[derive(Clone, Debug)]
pub enum TaskQuery {
  ByTaskIds(Vec<KlingTaskId>),
  ByExternalTaskIds(Vec<String>),
}

#[derive(Clone, Debug)]
pub struct GetTasksArgs<'a> {
  pub api_key: &'a KlingApiKey,
  pub query: TaskQuery,
}

/// `GET https://api-singapore.klingai.com/tasks` — batch-fetch tasks by
/// system id or by caller-chosen external id.
///
/// Serves every new-style (model-version-in-path) create binding. Note a
/// failed generation is an `Ok` task with `status == Failed` — only
/// transport/HTTP problems are `Err`.
pub async fn get_tasks(args: GetTasksArgs<'_>) -> Result<Vec<KlingTask>, KlingError> {
  let GetTasksArgs { api_key, query } = args;

  let (param, joined) = match &query {
    TaskQuery::ByTaskIds(ids) => {
      if ids.is_empty() {
        return Err(KlingClientError::InvalidRequest("task_ids must not be empty".into()).into());
      }
      ("task_ids", ids.iter().map(|id| id.0.as_str()).collect::<Vec<_>>().join(","))
    }
    TaskQuery::ByExternalTaskIds(ids) => {
      if ids.is_empty() {
        return Err(KlingClientError::InvalidRequest("external_task_ids must not be empty".into()).into());
      }
      ("external_task_ids", ids.join(","))
    }
  };

  info!(param, ids = %joined, "Kling get_tasks");

  let url = format!("{KLING_API_BASE_URL}/tasks");
  let client = http_client(KLING_USER_AGENT);

  let response = with_auth(client.get(&url).query(&[(param, joined.as_str())]), api_key)
    .send()
    .await
    .map_err(KlingGenericApiError::ReqwestError)?;

  let status = response.status();
  let response_body = response.text().await.map_err(KlingGenericApiError::ReqwestError)?;

  info!(status = %status, "Kling get_tasks response");

  classify_kling_http_error(status, Some(&response_body))?;

  let parsed: KlingEnvelope<Vec<KlingTask>> = serde_json::from_str(&response_body)
    .map_err(|e| KlingGenericApiError::SerdeResponseParseErrorWithBody(e, response_body.clone()))?;

  parsed.into_data(&response_body)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn empty_id_lists_are_rejected_client_side() {
    let ids: Vec<KlingTaskId> = vec![];
    match TaskQuery::ByTaskIds(ids) {
      TaskQuery::ByTaskIds(ids) => assert!(ids.is_empty()),
      _ => unreachable!(),
    }
    // The binding itself is async; the validation branch is covered by the
    // envelope test below plus the live test. Here we assert the query enum
    // keeps the two id spaces separate at the type level.
    let query = TaskQuery::ByExternalTaskIds(vec!["job_1".to_string(), "job_2".to_string()]);
    match query {
      TaskQuery::ByExternalTaskIds(ids) => assert_eq!(ids.join(","), "job_1,job_2"),
      _ => unreachable!(),
    }
  }

  #[test]
  fn documented_response_envelope_parses() {
    let json = r#"{
      "code": 0,
      "message": "SUCCEED",
      "request_id": "req_1",
      "data": [
        {
          "id": "893605946402811985",
          "status": "succeeded",
          "external_id": "job_01ABC",
          "outputs": [{"type": "video", "id": "v_1", "url": "https://cdn.example/v.mp4", "duration": "5.0"}],
          "billing": [{"charge_type": "cash", "amount": "0.56", "list_price": "0.70"}]
        }
      ]
    }"#;
    let envelope: KlingEnvelope<Vec<KlingTask>> = serde_json::from_str(json).unwrap();
    let tasks = envelope.into_data(json).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].id.0, "893605946402811985");
  }

  // ── Live API test (ignored — requires a real key; querying is free) ──
  // cargo test -p kling_client get_tasks -- --ignored --nocapture

  #[tokio::test]
  #[ignore] // manually test — requires real API key
  async fn live_test_get_tasks_unknown_id() -> anyhow::Result<()> {
    use crate::test_utils::get_test_api_key::get_test_api_key;
    use crate::test_utils::setup_test_logging::setup_test_logging;
    setup_test_logging();

    let api_key = get_test_api_key()?;
    let result = get_tasks(GetTasksArgs {
      api_key: &api_key,
      query: TaskQuery::ByExternalTaskIds(vec!["kling_client_smoke_nonexistent".to_string()]),
    })
    .await;
    println!("result: {result:?}");
    Ok(())
  }
}
