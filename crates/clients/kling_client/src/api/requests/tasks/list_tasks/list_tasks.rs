use client_core::http_client;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::api::requests::kling_host::{with_auth, KLING_API_BASE_URL, KLING_USER_AGENT};
use crate::api::types::task_types::kling_envelope::KlingEnvelope;
use crate::api::types::task_types::kling_task::KlingTask;
use crate::api::types::task_types::kling_task_status::KlingTaskStatus;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::classify_kling_http_error::classify_kling_http_error;
use crate::error::kling_client_error::KlingClientError;
use crate::error::kling_error::KlingError;
use crate::error::kling_generic_api_error::KlingGenericApiError;

/// Filters for [`list_tasks`]. Each variant maps to one `{key, values}` wire
/// entry.
#[derive(Clone, Debug)]
pub enum TaskFilter {
  Status(Vec<KlingTaskStatus>),
  ProductType(Vec<ProductType>),
}

/// Kling's `product_type` filter vocabulary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProductType {
  Video,
  Image,
  TryOn,
}

impl ProductType {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Video => "video",
      Self::Image => "image",
      Self::TryOn => "try_on",
    }
  }
}

/// Cursor-paged task listing. Timestamps are Unix milliseconds; when `cursor`
/// is set the time window is ignored (pass the previous page's `next_cursor`).
#[derive(Clone, Debug, Default)]
pub struct ListTasksRequest {
  /// Defaults server-side to `end_time` − 30 days.
  pub start_time: Option<i64>,
  /// Defaults server-side to now.
  pub end_time: Option<i64>,
  pub cursor: Option<String>,
  /// Default 100, max 500.
  pub limit: Option<u32>,
  pub filters: Vec<TaskFilter>,
}

#[derive(Clone, Debug)]
pub struct ListTasksArgs<'a> {
  pub api_key: &'a KlingApiKey,
  pub request: ListTasksRequest,
}

/// One page of results from [`list_tasks`].
#[derive(Clone, Debug, Deserialize)]
pub struct TaskPage {
  #[serde(default)]
  pub result: Vec<KlingTask>,
  #[serde(default)]
  pub count: Option<i64>,
  /// Pass back as `cursor` to fetch the next page.
  #[serde(default)]
  pub next_cursor: Option<String>,
  #[serde(default)]
  pub has_more: Option<bool>,
}

#[derive(Serialize)]
pub(crate) struct ListTasksBody {
  #[serde(skip_serializing_if = "Option::is_none")]
  start_time: Option<i64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  end_time: Option<i64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  cursor: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  limit: Option<u32>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  filters: Vec<FilterWire>,
}

#[derive(Serialize)]
pub(crate) struct FilterWire {
  key: &'static str,
  values: Vec<String>,
}

pub(crate) fn build_list_tasks_body(request: &ListTasksRequest) -> Result<ListTasksBody, KlingClientError> {
  if let Some(limit) = request.limit {
    if limit == 0 || limit > 500 {
      return Err(KlingClientError::InvalidRequest(format!("limit must be in 1..=500, got {limit}")));
    }
  }
  let filters = request
    .filters
    .iter()
    .map(|f| match f {
      TaskFilter::Status(statuses) => FilterWire {
        key: "status",
        values: statuses.iter().map(|s| s.as_str().to_string()).collect(),
      },
      TaskFilter::ProductType(kinds) => FilterWire {
        key: "product_type",
        values: kinds.iter().map(|k| k.as_str().to_string()).collect(),
      },
    })
    .collect();
  Ok(ListTasksBody {
    start_time: request.start_time,
    end_time: request.end_time,
    cursor: request.cursor.clone(),
    limit: request.limit,
    filters,
  })
}

/// `POST https://api-singapore.klingai.com/tasks` — list tasks by time window
/// / cursor, optionally filtered by status or product type.
pub async fn list_tasks(args: ListTasksArgs<'_>) -> Result<TaskPage, KlingError> {
  let ListTasksArgs { api_key, request } = args;

  let body = build_list_tasks_body(&request)?;
  let body_json = serde_json::to_string(&body).map_err(KlingClientError::RequestSerializationError)?;

  info!(limit = ?request.limit, has_cursor = request.cursor.is_some(), "Kling list_tasks");

  let url = format!("{KLING_API_BASE_URL}/tasks");
  let client = http_client(KLING_USER_AGENT);

  let response = with_auth(client.post(&url), api_key)
    .header("Content-Type", "application/json")
    .body(body_json)
    .send()
    .await
    .map_err(KlingGenericApiError::ReqwestError)?;

  let status = response.status();
  let response_body = response.text().await.map_err(KlingGenericApiError::ReqwestError)?;

  info!(status = %status, "Kling list_tasks response");

  classify_kling_http_error(status, Some(&response_body))?;

  let parsed: KlingEnvelope<TaskPage> = serde_json::from_str(&response_body)
    .map_err(|e| KlingGenericApiError::SerdeResponseParseErrorWithBody(e, response_body.clone()))?;

  parsed.into_data(&response_body)
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  #[test]
  fn wire_shape_with_filters() {
    let request = ListTasksRequest {
      start_time: Some(1_781_193_600_000),
      end_time: Some(1_781_516_352_968),
      cursor: None,
      limit: Some(500),
      filters: vec![
        TaskFilter::Status(vec![KlingTaskStatus::Succeeded]),
        TaskFilter::ProductType(vec![ProductType::Video]),
      ],
    };
    let body = build_list_tasks_body(&request).unwrap();
    assert_eq!(
      serde_json::to_value(&body).unwrap(),
      json!({
        "start_time": 1_781_193_600_000_i64,
        "end_time": 1_781_516_352_968_i64,
        "limit": 500,
        "filters": [
          {"key": "status", "values": ["succeeded"]},
          {"key": "product_type", "values": ["video"]}
        ]
      })
    );
  }

  #[test]
  fn empty_request_serializes_to_empty_object() {
    let body = build_list_tasks_body(&ListTasksRequest::default()).unwrap();
    assert_eq!(serde_json::to_value(&body).unwrap(), json!({}));
  }

  #[test]
  fn out_of_range_limit_is_rejected() {
    let request = ListTasksRequest { limit: Some(501), ..Default::default() };
    assert!(matches!(
      build_list_tasks_body(&request),
      Err(KlingClientError::InvalidRequest(_))
    ));
  }

  #[test]
  fn documented_page_response_parses() {
    let json = r#"{
      "result": [{"id": "1", "status": "processing"}],
      "count": 1,
      "next_cursor": "cursor_abc",
      "has_more": true
    }"#;
    let page: TaskPage = serde_json::from_str(json).unwrap();
    assert_eq!(page.result.len(), 1);
    assert_eq!(page.next_cursor.as_deref(), Some("cursor_abc"));
    assert_eq!(page.has_more, Some(true));
  }
}
