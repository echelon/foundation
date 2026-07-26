//! Shared plumbing for the legacy-style `/v1/*` endpoints (image, audio,
//! element and voice management): same Bearer auth and `{code, message,
//! request_id, data}` envelope as the new-style endpoints, but with the
//! legacy `task_status` vocabulary inside `data`.

use client_core::http_client;
use serde::de::DeserializeOwned;
use tracing::info;

use crate::api::requests::kling_host::{with_auth, KLING_API_BASE_URL, KLING_USER_AGENT};
use crate::api::types::task_types::kling_envelope::KlingEnvelope;
use crate::creds::kling_api_key::KlingApiKey;
use crate::error::classify_kling_http_error::classify_kling_http_error;
use crate::error::kling_client_error::KlingClientError;
use crate::error::kling_error::KlingError;
use crate::error::kling_generic_api_error::KlingGenericApiError;

/// Pagination for the legacy list endpoints. `page_num` ranges 1–1000;
/// `page_size` limits vary per endpoint (validated at the binding).
#[derive(Clone, Copy, Debug, Default)]
pub struct PageParams {
  /// Default 1.
  pub page_num: Option<u32>,
  /// Default 30.
  pub page_size: Option<u32>,
}

impl PageParams {
  pub(crate) fn validate(&self, max_page_size: u32) -> Result<(), KlingClientError> {
    if let Some(page_num) = self.page_num {
      if page_num == 0 || page_num > 1000 {
        return Err(KlingClientError::InvalidRequest(format!("pageNum must be in 1..=1000, got {page_num}")));
      }
    }
    if let Some(page_size) = self.page_size {
      if page_size == 0 || page_size > max_page_size {
        return Err(KlingClientError::InvalidRequest(format!(
          "pageSize must be in 1..={max_page_size}, got {page_size}"
        )));
      }
    }
    Ok(())
  }

  pub(crate) fn to_query(self) -> Vec<(&'static str, String)> {
    let mut query = Vec::new();
    if let Some(page_num) = self.page_num {
      query.push(("pageNum", page_num.to_string()));
    }
    if let Some(page_size) = self.page_size {
      query.push(("pageSize", page_size.to_string()));
    }
    query
  }
}

/// POST a pre-serialized JSON body to a legacy path and unwrap the envelope.
pub(crate) async fn legacy_post<T: DeserializeOwned>(
  api_key: &KlingApiKey,
  path: &str,
  body_json: String,
) -> Result<T, KlingError> {
  info!(path, "Kling legacy POST");

  let url = format!("{KLING_API_BASE_URL}{path}");
  let client = http_client(KLING_USER_AGENT);

  let response = with_auth(client.post(&url), api_key)
    .header("Content-Type", "application/json")
    .body(body_json)
    .send()
    .await
    .map_err(KlingGenericApiError::ReqwestError)?;

  let status = response.status();
  let response_body = response.text().await.map_err(KlingGenericApiError::ReqwestError)?;

  info!(status = %status, path, "Kling legacy POST response");

  classify_kling_http_error(status, Some(&response_body))?;

  let parsed: KlingEnvelope<T> = serde_json::from_str(&response_body)
    .map_err(|e| KlingGenericApiError::SerdeResponseParseErrorWithBody(e, response_body.clone()))?;

  parsed.into_data(&response_body)
}

/// GET a legacy path (with optional query parameters) and unwrap the envelope.
pub(crate) async fn legacy_get<T: DeserializeOwned>(
  api_key: &KlingApiKey,
  path: &str,
  query: &[(&'static str, String)],
) -> Result<T, KlingError> {
  info!(path, "Kling legacy GET");

  let url = format!("{KLING_API_BASE_URL}{path}");
  let client = http_client(KLING_USER_AGENT);

  let mut builder = client.get(&url);
  if !query.is_empty() {
    builder = builder.query(query);
  }

  let response = with_auth(builder, api_key)
    .send()
    .await
    .map_err(KlingGenericApiError::ReqwestError)?;

  let status = response.status();
  let response_body = response.text().await.map_err(KlingGenericApiError::ReqwestError)?;

  info!(status = %status, path, "Kling legacy GET response");

  classify_kling_http_error(status, Some(&response_body))?;

  let parsed: KlingEnvelope<T> = serde_json::from_str(&response_body)
    .map_err(|e| KlingGenericApiError::SerdeResponseParseErrorWithBody(e, response_body.clone()))?;

  parsed.into_data(&response_body)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn page_params_validate_ranges() {
    assert!(PageParams::default().validate(500).is_ok());
    assert!(PageParams { page_num: Some(1000), page_size: Some(500) }.validate(500).is_ok());
    assert!(PageParams { page_num: Some(0), page_size: None }.validate(500).is_err());
    assert!(PageParams { page_num: Some(1001), page_size: None }.validate(500).is_err());
    assert!(PageParams { page_num: None, page_size: Some(501) }.validate(500).is_err());
    // Voice lists allow up to 1000.
    assert!(PageParams { page_num: None, page_size: Some(501) }.validate(1000).is_ok());
  }

  #[test]
  fn page_params_to_query_omits_unset() {
    assert!(PageParams::default().to_query().is_empty());
    let query = PageParams { page_num: Some(2), page_size: Some(50) }.to_query();
    assert_eq!(query, vec![("pageNum", "2".to_string()), ("pageSize", "50".to_string())]);
  }
}
